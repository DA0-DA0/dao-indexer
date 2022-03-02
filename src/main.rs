use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
use cosmrs::proto::cosmos::base::v1beta1::Coin;
use cosmrs::proto::cosmwasm::wasm::v1::{MsgExecuteContract, MsgInstantiateContract};
use cosmrs::tx::{MsgProto, Tx};
use cw20_base::msg::InstantiateMarketingInfo;
use cw20::Cw20Coin;
use cw3_dao::msg::{GovTokenMsg, InstantiateMsg};
use dao_indexer::db::connection::establish_connection;
use dao_indexer::db::models::NewContract;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use futures::StreamExt;
use serde_json::Value;
use std::collections::BTreeMap;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};

fn parse_message(msg: &[u8]) -> serde_json::Result<Option<Value>> {
    if let Ok(exec_msg_str) = String::from_utf8(msg.to_owned()) {
        if let Ok(parsed_json) = serde_json::from_str(&exec_msg_str) {
            return Ok(parsed_json);
        }
    }
    Ok(None)
}

fn index_message(
    _db: &PgConnection,
    sender: &str,
    contract_addr: &str,
    funds: &[Coin],
    msg: Option<&Vec<u8>>,
) {
    let mut json_dump: String = "".to_string();
    if let Some(msg) = msg {
        if let Ok(Some(parsed)) = parse_message(msg) {
            let obj = parsed.as_object();
            json_dump = serde_json::to_string_pretty(&obj).unwrap();
        }
    }
    println!(
        "{{\"sender\": \"{}\", \"contract_address\": \"{}\", \"funds\": \"{:?}\", \"contract\": {}}}",
        sender,
        contract_addr,
        funds,
        json_dump
    );
}

trait Index {
    fn index(&self, db: &PgConnection, events: &Option<BTreeMap<String, Vec<String>>>);
}

impl Index for MsgExecuteContract {
    fn index(&self, db: &PgConnection, _events: &Option<BTreeMap<String, Vec<String>>>) {
        index_message(
            db,
            &self.sender,
            &self.contract,
            &self.funds,
            Some(&self.msg),
        )
    }
}

fn get_contract_address(events: &Option<BTreeMap<String, Vec<String>>>) -> String {
    let mut contract_addr = "".to_string();
    if let Some(transaction_events) = events {
        if let Some(addr) = transaction_events.get("instantiate._contract_address") {
            // This appears to be the correct address but why?
            contract_addr = addr[0].clone();
        }
    }
    contract_addr
}

fn insert_contract(db: &PgConnection, contract_model: &NewContract) {
    use dao_indexer::db::schema::contracts::dsl::*;
    diesel::insert_into(contracts)
        .values(contract_model)
        .execute(db)
        .expect("Error saving new post");
}

fn insert_marketing_info(
    db: &PgConnection,
    marketing_info: &InstantiateMarketingInfo,
) -> QueryResult<i32> {
    use dao_indexer::db::schema::marketing::dsl::*;
    diesel::insert_into(marketing)
        .values((
            project.eq(&marketing_info.project),
            description.eq(&marketing_info.description),
            marketing_text.eq(&marketing_info.marketing),
        ))
        .returning(id)
        .get_result(db)
}

fn update_balance(_db: &PgConnection, token_id: i32, balance: &Cw20Coin) {
    println!("TODO: update balance {}, {} for token id {}", balance.address, balance.amount, token_id);
}

fn insert_gov_token(db: &PgConnection, token_msg: &GovTokenMsg) -> QueryResult<i32> {
    use dao_indexer::db::schema::gov_token::dsl::*;
    let result: QueryResult<i32>;
    match token_msg {
        GovTokenMsg::InstantiateNewCw20 {
            /*cw20_code_id, stake_contract_code_id, label,*/ msg,
            ..
        } => {
            let mut marketing_record_id: Option<i32> = None;
            if let Some(marketing) = &msg.marketing {
                marketing_record_id = Some(insert_marketing_info(db, marketing).unwrap());
            }
            result = diesel::insert_into(gov_token)
                .values((
                    name.eq(&msg.name),
                    symbol.eq(&msg.symbol),
                    decimals.eq(msg.decimals as i32),
                    marketing_id.eq(marketing_record_id),
                ))
                .returning(id)
                .get_result(db);
            if let Ok(token_id) = result {
                for balance in &msg.initial_balances {
                    update_balance(db, token_id, balance);
                }
            }
        }
        GovTokenMsg::UseExistingCw20 {
            addr,
            stake_contract_code_id,
            label,
            unstaking_duration,
        } => {
            println!("TODO: Use existing cw20 addr: {}, stake_contract_code_id: {}, label: {}, unstaking_duration: {:?}", addr, stake_contract_code_id, label, unstaking_duration);
            result = Ok(0);
        }
    };
    result
}

fn insert_dao(db: &PgConnection, instantiate_dao: &InstantiateMsg, contract_addr: &str) {
    use dao_indexer::db::schema::dao::dsl::*;

    let inserted_token_id: i32 = insert_gov_token(db, &instantiate_dao.gov_token).unwrap();

    diesel::insert_into(dao)
        .values((
            name.eq(&instantiate_dao.name),
            contract_address.eq(&contract_addr),
            description.eq(&instantiate_dao.description),
            gov_token_id.eq(inserted_token_id),
        ))
        .execute(db)
        .expect("Error saving dao");
}

impl Index for MsgInstantiateContract {
    fn index(&self, db: &PgConnection, events: &Option<BTreeMap<String, Vec<String>>>) {
        let contract_addr = get_contract_address(events);
        let contract_model = NewContract {
            address: &contract_addr,
            admin: &self.admin,
            code_id: self.code_id as i64,
            creator: &self.sender,
            label: &self.label,
            creation_time: "",
            height: 0,
        };
        insert_contract(db, &contract_model);
        let msg_str = String::from_utf8(self.msg.clone()).unwrap();
        let instantiate_dao: InstantiateMsg = serde_json::from_str(&msg_str).unwrap();
        insert_dao(db, &instantiate_dao, &contract_addr);
    }
}

impl Index for MsgSend {
    fn index(&self, db: &PgConnection, _events: &Option<BTreeMap<String, Vec<String>>>) {
        index_message(db, &self.from_address, &self.to_address, &self.amount, None);
    }
}

#[tokio::main]
async fn main() {
    let db: PgConnection = establish_connection();
    let (client, driver) = WebSocketClient::new("ws://127.0.0.1:26657/websocket")
        .await
        .unwrap();
    let driver_handle = tokio::spawn(async move { driver.run().await });

    // Subscribe to transactions (can also add blocks but just Tx for now)
    let mut subs = client.subscribe(EventType::Tx.into()).await.unwrap();

    while let Some(res) = subs.next().await {
        let ev = res.unwrap();
        let result = ev.data;
        let events = ev.events;
        match result {
            EventData::NewBlock { block, .. } => println!("{:?}", block.unwrap()),
            EventData::Tx { tx_result, .. } => match Tx::from_bytes(&tx_result.tx) {
                Ok(tx_parsed) => {
                    for msg in tx_parsed.body.messages {
                        let type_url: &str = &msg.type_url;
                        match type_url {
                            "/cosmwasm.wasm.v1.MsgInstantiateContract" => {
                                let msg_obj: MsgInstantiateContract =
                                    MsgProto::from_any(&msg).unwrap();
                                msg_obj.index(&db, &events);
                            }
                            "/cosmwasm.wasm.v1.MsgExecuteContract" => {
                                let msg_obj: MsgExecuteContract = MsgProto::from_any(&msg).unwrap();
                                msg_obj.index(&db, &events);
                            }
                            "/cosmos.bank.v1beta1.MsgSend" => {
                                let msg_obj: MsgSend = MsgProto::from_any(&msg).unwrap();
                                msg_obj.index(&db, &events);
                            }
                            _ => {
                                println!("No handler for {}", type_url);
                            }
                        }
                    }
                }
                Err(err) => println!("ERROR: {:?}", err),
            },
            _ => println!("unexpected result"),
        }
    }

    // Signal to the driver to terminate.
    match client.close() {
        Ok(val) => println!("closed {:?}", val),
        Err(e) => println!("Error closing client {:?}", e),
    }
    // Await the driver's termination to ensure proper connection closure.
    let _ = driver_handle.await.unwrap();
}
