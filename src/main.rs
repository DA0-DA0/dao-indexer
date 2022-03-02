mod models;

use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
use cosmrs::proto::cosmos::base::v1beta1::Coin;
use cosmrs::tx::{MsgProto, Tx};
use dao_indexer_rs::db::connection::establish_connection;
use dao_indexer_rs::db::models::NewContract;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use futures::StreamExt;
use serde_json::Value;
use std::collections::BTreeMap;
use tendermint_rpc::event::{EventData};
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};
use cosmos_sdk_proto::cosmwasm::wasm::v1::{
    query_client::QueryClient as GrpcQueryClient
};
use tonic::transport::channel::Channel;
use cosmos_sdk_proto::cosmwasm::wasm::v1::MsgExecuteContract;
use cosmos_sdk_proto::cosmwasm::wasm::v1::MsgInstantiateContract;
use cosmos_sdk_proto::cosmwasm::wasm::v1::QuerySmartContractStateRequest;


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

impl Index for MsgInstantiateContract {
    fn index(&self, db: &PgConnection, events: &Option<BTreeMap<String, Vec<String>>>) {
        use dao_indexer_rs::db::schema::contracts::dsl::*;
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

        diesel::insert_into(contracts)
            .values(&contract_model)
            .execute(db)
            .expect("Error saving new post");

        index_message(
            db,
            &self.sender,
            &contract_addr,
            &self.funds,
            Some(&self.msg),
        )
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

    let mut grpc_client = cosmos_sdk_proto::cosmwasm::wasm::v1::query_client::QueryClient::connect("http://localhost:9090/")
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
                        let type_url = msg.type_url.to_string();
                        match type_url.as_str() {
                            "/cosmwasm.wasm.v1.MsgInstantiateContract" => {
                                let msg_obj: MsgInstantiateContract =
                                    MsgProto::from_any(&msg).unwrap();

                                let contract_addr = get_contract_address(&events);
                                let smart_contract_query_state = QuerySmartContractStateRequest {
                                    address: contract_addr.clone(),
                                    query_data: vec![],
                                };

                                let JSONGetContractInfo = models::QueryMsg::ContractInfo {};
                                // let y =  JSONGetContractInfo.serialize().unwrap();

                                let y = serde_json::to_string(&JSONGetContractInfo).unwrap();

                                println!("{}", y);

                                let response = grpc_client
                                    .smart_contract_state(smart_contract_query_state)
                                    .await.unwrap()
                                    .into_inner();

                                let result = String::from_utf8(response.data).unwrap();

                                println!("{}", result);

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
