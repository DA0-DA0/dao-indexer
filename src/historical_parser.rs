use crate::db::models::{Block, NewBlock};
use crate::indexer::tx::process_parsed;
use crate::util::history_util::tx_to_hash;
use cosmrs::tx::Tx;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::collections::BTreeMap;
use std::collections::HashSet;
use tendermint::abci::responses::Event;
use tendermint_rpc::Client;
use tendermint_rpc::HttpClient as TendermintClient;

fn map_from_events(
    events: &Vec<Event>,
    event_map: &mut BTreeMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    for event in events {
        let event_name = &event.type_str;
        for attribute in &event.attributes {
            let attributes;
            let attribute_key: &str = &attribute.key.to_string();
            let event_key = format!("{}.{}", event_name, attribute_key);
            if let Some(existing_attributes) = event_map.get_mut(&event_key) {
                attributes = existing_attributes;
            } else {
                event_map.insert(event_key.clone(), vec![]);
                attributes = event_map
                    .get_mut(&event_key)
                    .ok_or(format!("no attribute {} found", event_key))?;
            }
            attributes.push(attribute.value.to_string());
        }
    }
    Ok(())
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
            let results = tendermint_client
                .block_results(block_height as u32)
                .await
                .unwrap();
            let mut all_events = BTreeMap::<String, Vec<String>>::default();
            all_events.insert("tx.height".to_string(), vec![format!("{}", block_height)]);
            if let Some(txs_results) = results.txs_results {
                for tx in txs_results {
                    map_from_events(&tx.events, &mut all_events).unwrap();
                }
            }

            let response = tendermint_client.block(block_height as u32).await.unwrap();
    
            let block_hash = response.block_id.hash.to_string();
            if save_all_blocks {
                let new_block = NewBlock::from_block_response(&block_hash, &response.block);
                diesel::insert_into(block)
                    .values(&new_block)
                    .execute(db)
                    .expect("Error saving new Block");
            }
            // Look at the transactions:
            for tx in response.block.data.iter() {
                let tx_hash = tx_to_hash(tx);
                let tx_response = tendermint_client.tx(tx_hash, false).await.unwrap();
                let mut events = BTreeMap::<String, Vec<String>>::default();
                events.insert("tx.height".to_string(), vec![block_height.to_string()]);
                let _ = map_from_events(&tx_response.tx_result.events, &mut events);
                let unmarshalled_tx = Tx::from_bytes(tx.as_bytes()).unwrap();
                let _ = process_parsed(db, &unmarshalled_tx, &Some(events));
            }
        }
    }
}

pub fn classify_transaction(tx: cosmrs::Any) {
    match tx.type_url.to_string().as_str() {
        "/cosmwasm.wasm.v1.MsgInstantiateContract" => {
            println!("we found an instnatiate contract, p0g")
        }
        _ => {
            println!("No handler for {}", tx.type_url.to_string().as_str());
        }
    }
}