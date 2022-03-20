use crate::db::models::{Block, NewBlock};
use crate::indexer::tx::process_parsed;
use cosmos_sdk_proto::cosmos::tx::v1beta1::TxRaw;
use cosmrs::tx::Tx;
// use cosmrs::tx::Tx;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::collections::HashSet;
use tendermint_rpc::Client;
use tendermint_rpc::HttpClient as TendermintClient;
use tendermint::abci::responses::Event;
use std::collections::BTreeMap;
use serde_json::{Result, Value};
use cosmos_sdk_proto::COSMOS_SDK_VERSION;

fn map_from_events(events: &Vec<Event>) -> BTreeMap::<String, Vec<String>> {
    let mut event_map = BTreeMap::<String, Vec<String>>::default();
    for event in events {
        let mut event_strings = vec!();
        for attribute in &event.attributes {
            println!("discarding event attribute key {}", attribute.key.to_string());
            event_strings.push(attribute.value.to_string())
        }
        event_map.insert(event.type_str.clone(), event_strings);
    }
    event_map
}

pub async fn block_synchronizer(
    db: &PgConnection,
    tendermint_rpc_url: &str,
    initial_block_height: u64,
    save_all_blocks: bool,
) {
    use crate::db::schema::block::dsl::*;
    let tendermint_client = TendermintClient::new(tendermint_rpc_url).unwrap();

    let latest_block_response = tendermint_client.latest_block_results().await.unwrap();
    let latest_block_height = latest_block_response.height.value();

    let blocks_from_db = block.load::<Block>(db).unwrap();

    let mut set_of_already_indexed_blocks = HashSet::new();

    // TODO(gavindoughtie): This is inefficient. We should just hit the DB for this.
    for db_block in blocks_from_db {
        set_of_already_indexed_blocks.insert(db_block.height);
    }

    for block_height in initial_block_height..latest_block_height {
        if !set_of_already_indexed_blocks.contains(&(block_height as i64)) {
            if block_height % 1000 == 0 {
                println!("Added another 1000 blocks, height: {}", block_height);
            }

            // TODO(entrancedjames): This should be a join instead of done in series.
            let response = tendermint_client.block(block_height as u32).await.unwrap();
            let abci_response = tendermint_client.block_results(block_height as u32).await.unwrap();

            let block_hash = response.block_id.hash.to_string();
            if save_all_blocks {
                let new_block = NewBlock::from_block_response(&block_hash, &response.block);
                diesel::insert_into(block)
                    .values(&new_block)
                    .execute(db)
                    .expect("Error saving new Block");
            }
            use cosmrs::tx::Raw;
            use sha2::{Sha256, Digest};

            for tx in response.block.data.iter() {
                let z = Raw::from_bytes(tx.as_bytes()).unwrap();
                let kappa = z.to_bytes().unwrap();
                let omega = Sha256::digest(&kappa);
            }

            // if (response.block.data.iter().count() != abci_response.txs_results.unwrap_or_default().len()) {
            //     println!("Not equal!");
            // }

            // println!("Block TX Size: {}", response.block.data.iter().count());
            // println!("ABCI TX Size :{}", abci_response.txs_results.unwrap_or_default().len());
            // serde_json::
            // let tx_events = abci_response.txs_results.unwrap_or_default(); // default to empty sequence
            // // println("Event Size Txs and ")
            // for tx_with_events in tx_events {
            //     // println!("{}",)
            //     // tx_with_events.data
            //     let zeta = Tx::from_bytes(tx_with_events.data.value()).unwrap();

            //     TxRaw::from(zeta);
            //     // zeta.to_raw();
            //     // let msg_str = String::from(&tx_with_events.data.value()).unwrap();
            //     // base64::deco
            //     // println!("Data {}", tx_with_events.data);
            //     // println!("Decoded string: {}", msg_str);
            //     println!("Log {}", tx_with_events.log);

            //     let parsed_json: Value = serde_json::from_str(&tx_with_events.log.value()).unwrap();
            //     let contract_method_type = &parsed_json[0]["events"][0]["attributes"][0]["value"];

            //     let events = map_from_events(&tx_with_events.events);
            //     // let _ = process_messages(db, &messages, &Some(events));
            // }
        }
    }
}

fn classify_transaction(tx: cosmrs::Any) {
    match tx.type_url.to_string().as_str() {
        "/cosmwasm.wasm.v1.MsgInstantiateContract" => {
            println!("we found an instnatiate contract, p0g")
        }
        _ => {
            println!("No handler for {}", tx.type_url.to_string().as_str());
        }
    }
}
