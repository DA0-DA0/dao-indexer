use crate::db::models::{Block, NewBlock};
use crate::indexer::tx::{process_messages};
use cosmrs::tx::Tx;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::collections::HashSet;
use tendermint_rpc::Client;
use tendermint_rpc::HttpClient as TendermintClient;
use tendermint::abci::responses::Event;
use std::collections::BTreeMap;

fn map_from_events(events: &Vec<Event>) -> BTreeMap::<String, Vec<String>> {
    let mut attribute_map = BTreeMap::<String, Vec<String>>::default();
    for event in events {
        for attribute in &event.attributes {
            let attributes;
            let attribute_key: &str = &attribute.key.to_string();
            if let Some(existing_attributes) = attribute_map.get_mut(attribute_key) {
                attributes = existing_attributes;
            } else {
                attribute_map.insert(attribute.key.to_string(), vec![]);
                attributes = attribute_map.get_mut(attribute_key).unwrap();
            }
            attributes.push(attribute.value.to_string());          
        }
    }
    attribute_map
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
                let cosm_tx = Tx::from_bytes(tx.as_bytes()).unwrap();
                for msg in cosm_tx.body.messages {
                    println!("cosm_msg: {:?}", msg);
                }
            }
            
            let results = tendermint_client.block_results(block_height as u32).await.unwrap();
            if let Some(txs_results) = results.txs_results {
                for tx in txs_results {
                    let events = map_from_events(&tx.events);
                    if !events.is_empty() {
                        println!("Processed into event map: {:?}", events);
                        let messages = vec!();
                        let _ = process_messages(db, &messages, &Some(events));
                    }
                }
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
