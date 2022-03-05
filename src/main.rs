use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
use cosmrs::proto::cosmos::base::v1beta1::Coin;
// use cosmrs::proto::cosmwasm::wasm::v1::{MsgExecuteContract, MsgInstantiateContract};
use cosmos_sdk_proto::cosmwasm::wasm::v1::{
    // query_client::QueryClient as GrpcQueryClient,
    MsgExecuteContract,
    MsgInstantiateContract,
    // QuerySmartContractStateRequest
};
use cosmrs::tx::{MsgProto, Tx};
use cw20::Cw20Coin;
use cw20_base::msg::InstantiateMarketingInfo;
use cw3_dao::msg::{ExecuteMsg, GovTokenMsg, InstantiateMsg};
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

#[derive(Debug)]
struct ContractAddresses {
    dao_address: Option<String>,
    cw20_address: Option<String>,
    staking_contract_address: Option<String>,
}

fn get_contract_addresses(events: &Option<BTreeMap<String, Vec<String>>>) -> ContractAddresses {
    let mut dao_address = None;
    let mut cw20_address = None;
    let mut staking_contract_address = None;
    if let Some(transaction_events) = events {
        if let Some(addr) = transaction_events.get("instantiate._contract_address") {
            // 0: DAO
            // 1: cw20
            // 2: staking contract
            // But if you use an existing token, you'll just get
            // DAO/staking contract
            dao_address = Some(addr[0].clone());
            cw20_address = Some(addr[1].clone());
            staking_contract_address = Some(addr[2].clone());
        }
    }
    ContractAddresses {
        dao_address,
        cw20_address,
        staking_contract_address,
    }
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

fn update_balance(
    db: &PgConnection,
    token_id: i32,
    token_addr: &str,
    token_sender_address: &str,
    balance_update: &Cw20Coin,
) -> QueryResult<i32> {
    use dao_indexer::db::schema::cw20_transactions::dsl::*;
    println!(
        "TODO: update balance {}, {} for token id {}",
        balance_update.address, balance_update.amount, token_id
    );
    // Find an existing record for balance_udpate.address AND the token (id?? Address?)
    // If no existing record, insert one and we're done
    // If there is an existing record:
    //   deduct from the source
    //   add the new balance to the row
    // cw20_address TEXT NOT NULL,
    // sender_address TEXT NOT NULL,
    // recipient_address TEXT NOT NULL,
    // amount BIGINT NOT NULL

    diesel::insert_into(cw20_transactions)
        .values((
            cw20_address.eq(token_addr),
            sender_address.eq(token_sender_address),
            recipient_address.eq(&balance_update.address),
            amount.eq(balance_update.amount.u128() as i64), // Bigger data type?
        ))
        .returning(id)
        .get_result(db)
}

fn insert_gov_token(
    db: &PgConnection,
    token_msg: &GovTokenMsg,
    contract_addresses: &ContractAddresses,
) -> QueryResult<i32> {
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
            let cw20_address = contract_addresses.cw20_address.as_ref().unwrap();
            result = diesel::insert_into(gov_token)
                .values((
                    name.eq(&msg.name),
                    address.eq(cw20_address),
                    symbol.eq(&msg.symbol),
                    decimals.eq(msg.decimals as i32),
                    marketing_id.eq(marketing_record_id),
                ))
                .returning(id)
                .get_result(db);
            let dao_address = contract_addresses.dao_address.as_ref().unwrap();
            if let Ok(token_id) = result {
                for balance in &msg.initial_balances {
                    let _ = update_balance(db, token_id, cw20_address, dao_address, balance);
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

fn insert_dao(
    db: &PgConnection,
    instantiate_dao: &InstantiateMsg,
    contract_addr: &ContractAddresses,
) {
    use dao_indexer::db::schema::dao::dsl::*;

    let inserted_token_id: i32 =
        insert_gov_token(db, &instantiate_dao.gov_token, contract_addr).unwrap();

    diesel::insert_into(dao)
        .values((
            name.eq(&instantiate_dao.name),
            contract_address.eq(contract_addr.dao_address.as_ref().unwrap()),
            description.eq(&instantiate_dao.description),
            gov_token_id.eq(inserted_token_id),
        ))
        .execute(db)
        .expect("Error saving dao");
}

impl Index for MsgInstantiateContract {
    fn index(&self, db: &PgConnection, events: &Option<BTreeMap<String, Vec<String>>>) {
        let contract_addresses = get_contract_addresses(events);
        println!(
            "cw20 {}, staking {}",
            contract_addresses.cw20_address.as_ref().unwrap(),
            contract_addresses
                .staking_contract_address
                .as_ref()
                .unwrap()
        );
        let dao_address = contract_addresses.dao_address.as_ref().unwrap();
        let contract_model = NewContract {
            address: dao_address,
            admin: &self.admin,
            code_id: self.code_id as i64,
            creator: &self.sender,
            label: &self.label,
            creation_time: "",
            height: 0,
        };
        insert_contract(db, &contract_model);
        let msg_str = String::from_utf8(self.msg.clone()).unwrap();
        match serde_json::from_str::<InstantiateMsg>(&msg_str) {
            Ok(instantiate_dao) => {
                insert_dao(db, &instantiate_dao, &contract_addresses);
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        };
    }
}

fn dump_execute_contract(execute_contract: &ExecuteMsg) {
    println!("handle execute contract {:?}", execute_contract);
}

fn dump_events(events: &Option<BTreeMap<String, Vec<String>>>) {
    if let Some(event_map) = events {
        println!("************* vv Events ***********");
        for (key, value) in event_map {
            println!("{} / {:?}", key, value);
        }
        println!("************* ^^ Events ***********");
    }
}

impl Index for MsgExecuteContract {
    fn index(&self, _db: &PgConnection, events: &Option<BTreeMap<String, Vec<String>>>) {
        let msg_str = String::from_utf8(self.msg.clone()).unwrap();
        match serde_json::from_str::<ExecuteMsg>(&msg_str) {
            Ok(execute_contract) => {
                dump_execute_contract(&execute_contract);
                dump_events(events);
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        };
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
        print!("events: {:?}", events);
        match result {
            EventData::NewBlock { block, .. } => println!("{:?}", block.unwrap()),
            EventData::Tx { tx_result, .. } => match Tx::from_bytes(&tx_result.tx) {
                Ok(tx_parsed) => {
                    for msg in tx_parsed.body.messages {
                        let type_url: &str = &msg.type_url;
                        println!("instantiate msg: {}", type_url);
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
