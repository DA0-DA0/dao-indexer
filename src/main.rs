use std::collections::BTreeMap;

use cosmos_sdk_proto::cosmos::tx::v1beta1::service_client::ServiceClient;
use cosmos_sdk_proto::cosmwasm::wasm::v1::{
    MsgExecuteContract,
    MsgInstantiateContract,
    query_client::QueryClient as GrpcQueryClient,
    QuerySmartContractStateRequest
};
use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
use cosmrs::proto::cosmos::base::v1beta1::Coin;
use cosmrs::tx::{MsgProto, Tx};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use futures::StreamExt;
use serde_json::Value;
use tendermint::hash::Hash;
use tendermint_rpc::{HttpClient as TendermintClient, SubscriptionClient, WebSocketClient};
use tendermint_rpc::Client;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tonic::transport::Channel;
use dao_indexer_rs::db::connection::establish_connection;
use dao_indexer_rs::db::models::NewContract;
use dao_indexer_rs::db::models::NewBlock;


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

/*
create a table for blocks
- each block has a pkey, monotonically increasing
- each block has a height
- each block has a hash stored as string
- timestamp of the confirmed block
- num of txs
*/

/*
create a table of transactions...
TODO add information relevant.
How do we transform into comswasm messages?
*/

/*
Simple algorithm. Get first node, at the start.
Go each block, look at each transaction. store it in memory, perform some action per transaction.
for each tx then we can implement our own logic/custom code. this includes querying and indexing async for node information.
there is a retry sequence we want to do and also the batch sizes for each individual set of blocks.
*/


// fn create_new_blocK()

#[tokio::main]
async fn main() {
    let db: PgConnection = establish_connection();
    let (client, driver) = WebSocketClient::new("ws://127.0.0.1:26657/websocket")
        .await
        .unwrap();
    let tendermint_client = TendermintClient::new("http://127.0.0.1:26657").unwrap();
    for block_height in 1..10 {
        let response = tendermint_client.block( block_height as u32).await.unwrap();
        println!("{}", response.block_id.hash);

        let new_block = NewBlock {
            height: response.block.header.height.value() as i64,
            hash: &response.block_id.hash.to_string(),
            num_txs: response.block.data.iter().len() as i64,
        };
        use dao_indexer_rs::db::schema::block::dsl::*;

        diesel::insert_into(block)
            .values(&new_block)
            .execute(&db)
            .expect("Error saving new Block");


        for tx in response.block.data.iter() {
            let unmarshalled_tx = Tx::from_bytes(tx.as_bytes()).unwrap();
            for tx_message in unmarshalled_tx.body.messages {
                match tx_message.type_url.to_string().as_str() {
                    // String { .. } => {}
                    "/cosmwasm.wasm.v1.MsgInstantiateContract" => {
                        println!("we found an instnatiate contract, p0g")
                    }
                    _ => {
                        println!("No handler for {}", tx_message.type_url.to_string().as_str());
                    }
                }
            }
        }
    }

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

                                // let get_contract_info = models::QueryMsg::ContractInfo {};
                                // let serialized_get_contract_info = serde_json::to_vec(&get_contract_info).unwrap();
                                // let smart_contract_query_state = QuerySmartContractStateRequest {
                                //     address: contract_addr.clone(),
                                //     query_data: serialized_get_contract_info,
                                // };
                                //
                                // grpc_client
                                //     .contract_info()
                                //
                                // let response = grpc_client
                                //     .smart_contract_state(smart_contract_query_state)
                                //     .await.unwrap()
                                //     .into_inner();
                                //
                                // let result = String::from_utf8(response.data).unwrap();
                                //
                                // println!("{}", result);

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
