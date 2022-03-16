use bigdecimal::BigDecimal;
use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
use cosmrs::proto::cosmos::base::v1beta1::Coin;
use cosmrs::proto::cosmwasm::wasm::v1::{MsgExecuteContract, MsgInstantiateContract};
use cosmrs::tx::{MsgProto, Tx};
use cosmwasm_std::Uint128;
use cw20::Cw20Coin;
pub use cw20::Cw20ExecuteMsg;
use cw20_base::msg::InstantiateMarketingInfo;
use cw3_dao::msg::{
    ExecuteMsg as Cw3DaoExecuteMsg, GovTokenMsg, InstantiateMsg as Cw3DaoInstantiateMsg,
};
use dao_indexer::db::connection::establish_connection;
use dao_indexer::db::models::{Cw20, Dao, NewContract, NewDao, NewGovToken};
use dao_indexer::historical_parser::block_synchronizer;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use futures::StreamExt;
use serde_json::Value;
use stake_cw20::msg::ExecuteMsg as StakeCw20ExecuteMsg;
use std::collections::BTreeMap;
use std::str::FromStr;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};
use dotenv::dotenv;
use std::env;

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

fn get_dao(db: &PgConnection, dao_address: &str) -> QueryResult<Dao> {
    use dao_indexer::db::schema::dao::dsl::*;
    dao.filter(contract_address.eq(dao_address))
        .first::<Dao>(db)
}

fn get_gov_token(db: &PgConnection, dao_address: &str) -> diesel::QueryResult<Cw20> {
    use dao_indexer::db::schema::gov_token::dsl::*;
    let dao = get_dao(db, dao_address).unwrap();
    gov_token.filter(id.eq(dao.gov_token_id)).first(db)
}

fn dump_execute_contract(execute_contract: &Cw3DaoExecuteMsg) {
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

fn update_balance_from_events(
    db: &PgConnection,
    i: usize,
    event_map: &BTreeMap<String, Vec<String>>,
) -> QueryResult<usize> {
    let tx_height_string = &event_map.get("tx.height").unwrap()[0];
    let tx_height = BigDecimal::from_str(tx_height_string).unwrap();
    let amount = &event_map.get("wasm.amount").unwrap()[i];
    let receiver = &event_map.get("wasm.to").unwrap()[i];
    let sender = &event_map.get("wasm.sender").unwrap()[0];
    let from = &event_map.get("wasm.from").unwrap()[0]; // DAO address
    let gov_token = get_gov_token(db, from).unwrap();
    let balance_update = Cw20Coin {
        address: receiver.clone(),
        amount: Uint128::from_str(amount).unwrap(),
    };
    update_balance(
        db,
        Some(&tx_height),
        &gov_token.address,
        sender,
        &balance_update,
    )
}

impl Index for Cw3DaoExecuteMsg {
    fn index(&self, db: &PgConnection, events: &Option<BTreeMap<String, Vec<String>>>) {
        dump_execute_contract(self);
        dump_events(events);
        if let Some(event_map) = events {
            if let Some(wasm_actions) = event_map.get("wasm.action") {
                // TODO(gavin.doughtie): Handle propose, vote
                if !wasm_actions.is_empty() && wasm_actions[0] == "execute" {
                    for (i, action_type) in (&wasm_actions[1..]).iter().enumerate() {
                        match action_type.as_str() {
                            "transfer" => {
                                update_balance_from_events(db, i, event_map).unwrap();
                            }
                            "mint" => {
                                update_balance_from_events(db, i, event_map).unwrap();
                            }
                            _ => {
                                eprintln!("Unhandled exec type {}", action_type);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Index for StakeCw20ExecuteMsg {
    fn index(&self, _db: &PgConnection, events: &Option<BTreeMap<String, Vec<String>>>) {
        println!("StakeCw20ExecuteMsg index");
        dump_events(events);
    }
}

impl Index for Cw20ExecuteMsg {
    fn index(&self, db: &PgConnection, events: &Option<BTreeMap<String, Vec<String>>>) {
        dump_events(events);
        if let Some(event_map) = events {
            if let Some(wasm_actions) = event_map.get("wasm.action") {
                if !wasm_actions.is_empty() && &wasm_actions[0] == "send" {
                    let tx_height =
                        BigDecimal::from_str(&(event_map.get("tx.height").unwrap()[0])).unwrap();
                    let contract_addresses = event_map.get("wasm._contract_address").unwrap();
                    let gov_token_address = &contract_addresses[0];
                    let to_addresses = event_map.get("wasm.to").unwrap();
                    let staking_contract_addr = to_addresses[0].clone();
                    let amounts = &event_map.get("wasm.amount").unwrap();
                    let senders = event_map.get("wasm.from").unwrap();
                    let sender_addr = &senders[0];
                    let mut send_amount: &str = &amounts[0];

                    let receiving_contract_action: &str;
                    if wasm_actions.len() > 1 {
                        receiving_contract_action = &wasm_actions[1];
                    } else {
                        receiving_contract_action = "";
                    }
                    let action_amount: &str = &amounts[1];
                    if receiving_contract_action == "stake" {
                        send_amount = action_amount;
                    }
                    let balance_update: Cw20Coin = Cw20Coin {
                        address: staking_contract_addr,
                        amount: Uint128::from_str(send_amount).unwrap(),
                    };
                    let _ = update_balance(
                        db,
                        Some(&tx_height),
                        gov_token_address,
                        sender_addr,
                        &balance_update,
                    );
                }
            }
        }
    }
}

impl Index for MsgExecuteContract {
    fn index(&self, db: &PgConnection, events: &Option<BTreeMap<String, Vec<String>>>) {
        let msg_str = String::from_utf8(self.msg.clone()).unwrap();
        let mut errors = vec![];
        match serde_json::from_str::<Cw3DaoExecuteMsg>(&msg_str) {
            Ok(execute_contract) => {
                return execute_contract.index(db, events);
            }
            Err(e) => {
                errors.push(e);
            }
        };
        match serde_json::from_str::<StakeCw20ExecuteMsg>(&msg_str) {
            Ok(execute_contract) => {
                return execute_contract.index(db, events);
            }
            Err(e) => {
                errors.push(e);
            }
        };
        match serde_json::from_str::<Cw20ExecuteMsg>(&msg_str) {
            Ok(execute_contract) => {
                return execute_contract.index(db, events);
            }
            Err(e) => {
                errors.push(e);
            }
        }
        eprintln!("could not interpret execute msg, got errors:\n{:?}", errors);
    }
}

impl Index for MsgSend {
    fn index(&self, db: &PgConnection, _events: &Option<BTreeMap<String, Vec<String>>>) {
        index_message(db, &self.from_address, &self.to_address, &self.amount, None);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db: PgConnection = establish_connection();
    let (client, driver) = WebSocketClient::new("ws://127.0.0.1:26657/websocket")
        .await
        .unwrap();
    let driver_handle = tokio::spawn(async move { driver.run().await });


    dotenv().ok();

    let enable_indexer_env = env::var("ENABLE_INDEXER").unwrap_or("false".to_string());

    if enable_indexer_env == "true" {
        block_synchronizer(&db).await;
    } else {
        println!("Not indexing");
    }
    
    // Subscribe to transactions (can also add blocks but just Tx for now)
    let mut subs = client.subscribe(EventType::Tx.into()).await?;

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
                                eprintln!("No handler for {}", type_url);
                            }
                        }
                    }
                }
                Err(err) => eprintln!("{:?}", err),
            },
            _ => eprintln!("unexpected result"),
        }
    }

    // Signal to the driver to terminate.
    match client.close() {
        Ok(val) => println!("closed {:?}", val),
        Err(e) => eprintln!("Error closing client {:?}", e),
    }
    // Await the driver's termination to ensure proper connection closure.
    let _ = driver_handle.await.unwrap();

    Ok(())
}
