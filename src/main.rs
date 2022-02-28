use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
use cosmrs::proto::cosmos::base::v1beta1::Coin;
use cosmrs::proto::cosmwasm::wasm::v1::{MsgExecuteContract, MsgInstantiateContract};
use cosmrs::tx::{MsgProto, Tx};
use dao_indexer::db::connection::establish_connection;
use dao_indexer::db::models::{NewContract};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use futures::StreamExt;
use serde_json::Value;
use std::collections::BTreeMap;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};
use cw3_dao::msg::{InstantiateMsg, GovTokenMsg};

fn parse_message(msg: &Vec<u8>) -> serde_json::Result<Option<Value>> {
    if let Ok(exec_msg_str) = String::from_utf8(msg.clone()) {
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
    funds: &Vec<Coin>,
    msg: Option<&Vec<u8>>,
) {
    let mut json_dump: String = "".to_string();
    if let Some(msg) = msg {
        if let Ok(parsed) = parse_message(msg) {
            if let Some(parsed) = parsed {
                let obj = parsed.as_object().clone();
                json_dump = serde_json::to_string_pretty(&obj).unwrap();
            }
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
    return contract_addr;
}

fn insert_contract(db: &PgConnection, contract_model: &NewContract) {
    use dao_indexer::db::schema::contracts::dsl::*;
    diesel::insert_into(contracts)
    .values(contract_model)
    .execute(db)
    .expect("Error saving new post");
}

fn insert_dao(db: &PgConnection, instantiate_dao: &InstantiateMsg, contract_addr: &str) {
    use dao_indexer::db::schema::dao::dsl::*;

    let dao_cw20_code_id;
    let dao_stake_contract_code_id;
    let dao_label;
    let mut symbol = "NO_SYMBOL".to_string();
    match &instantiate_dao.gov_token {
        GovTokenMsg::InstantiateNewCw20{cw20_code_id, stake_contract_code_id, label, msg, ..} => {
            dao_cw20_code_id = cw20_code_id;
            dao_stake_contract_code_id = stake_contract_code_id;
            dao_label = label;
            symbol = msg.symbol.to_string();
            println!("dao_cw20_code_id {}", dao_cw20_code_id);
        },
        GovTokenMsg::UseExistingCw20{stake_contract_code_id, label, ..} => {
            dao_stake_contract_code_id = stake_contract_code_id;
            dao_label = label;
        }
    };

    print!("dao_stake_contract_code_id: {}", dao_stake_contract_code_id);

    diesel::insert_into(dao)
    .values((
        name.eq(&instantiate_dao.name),
        contract_address.eq(&contract_addr),
        description.eq(&instantiate_dao.description),
        token_name.eq(&dao_label),
        token_symbol.eq(symbol)
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
        // let code_id = match instantiate_dao.gov_token {
        //     GovTokenMsg::InstantiateNewCw20{cw20_code_id, ..} => {
        //         cw20_code_id as u64
        //         // println!("new cw20 {:?}", instantiate_dao.gov_token.label);
        //     },
        //     GovTokenMsg::UseExistingCw20{stake_contract_code_id, ..} => {
        //         // println!("existing cw20 {:?}", ..);
        //         stake_contract_code_id
        //     }
        // };
        // diesel::insert_into(dao)
        // .values((
        //     name.eq(&instantiate_dao.name),
        //     description.eq(&instantiate_dao.description)
        // ))
        // .execute(db)
        // .expect("Error saving dao");
        // if let Ok(parsed) = parse_message(&self.msg) {
        //     if let Some(parsed) = parsed {
        //         let description = parsed.get("description").unwrap();
        //         let gov_token = parsed.get("gov_token").unwrap();
        //         let cw20 = gov_token.get("instantiate_new_cw20").unwrap();
        //         let code_id_value = cw20.get("cw20_code_id").unwrap();
        //         let initial_dao_balance = cw20.get("initial_dao_balance").unwrap().as_str().unwrap();
        //         let token_label = cw20.get("label").unwrap().as_str().unwrap();
        //         let msg = cw20.get("msg").unwrap();
        //         let token_symbol = msg.get("symbol").unwrap().as_str().unwrap();
        //         let initial_dao_balances = msg.get("initial_balances").unwrap().as_array().unwrap();
        //         println!("{} {}, {} {} {}", description, initial_dao_balance, token_label, token_symbol, code_id_value);
        //         for balance in initial_dao_balances {
        //             println!("balance: {:?}", balance);
        //         }

        //         //println!("description: {}, gov_token: {}, cw20: {}, code_id: {}", description, gov_token, cw20, code_id_value);
        //         //println!("initial_dao_balance: {}, token_label: {}, token_symbol: {}, initial_dao_balances: {:?}", initial_dao_balance, token_label, token_symbol, initial_dao_balances);

        //         // let dao = NewDao {
        //         //     parsed.get("description");

        //         // }
        //     }
        // }

        // index_message(
        //     db,
        //     &self.sender,
        //     &contract_addr,
        //     &self.funds,
        //     Some(&self.msg),
        // )
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
