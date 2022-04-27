use crate::db::models::{Block, NewBlock};
use crate::db::schema::block::dsl::*;
use crate::indexing::indexer_registry::IndexerRegistry;
use crate::indexing::tx::process_parsed;
use crate::util::history_util::tx_to_hash;
use cosmrs::tx::{Tx, Hash};
use diesel::prelude::*;
use log::{error, info};
use std::collections::BTreeMap;
use tendermint::abci::responses::Event;
use tendermint_rpc::Client;
use tendermint_rpc::query::Query;
use tendermint_rpc::HttpClient as TendermintClient;
use std::collections::HashSet;

fn map_from_events(
    events: &[Event],
    event_map: &mut BTreeMap<String, Vec<String>>, // TODO(gavin.doughtie): type alias for the event map
) -> anyhow::Result<()> {
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
                    .ok_or_else(|| anyhow::anyhow!("no attribute {} found", event_key))?;
            }
            attributes.push(attribute.value.to_string());
        }
    }
    Ok(())
}

pub async fn block_synchronizer(
    registry: &IndexerRegistry,
    tendermint_rpc_url: &str,
    initial_block_height: u64,
    save_all_blocks: bool,
    transaction_page_size: u8,
    msg_set: &mut HashSet<String>
) -> anyhow::Result<()> {
    let tendermint_client = TendermintClient::new(tendermint_rpc_url)?;

    // let query = Query::gt("tx.height", 1_u64);
    // // let query: Query = "tx.height > 1".parse().unwrap();
    // let search_results = tendermint_client.tx_search(
    //     query,
    //     false,
    //     1,
    //     2,
    //     tendermint_rpc::Order::Ascending
    // ).await?;
    // println!("search_results: {:?}", search_results.total_count);

    // let parsed_query: Query = "tx.height > 1".parse().unwrap();
    // let search_results = tendermint_client.tx_search(
    //     parsed_query,
    //     false,
    //     1,
    //     2,
    //     tendermint_rpc::Order::Ascending
    // ).await?;
    // println!("search_results from parsed: {:?}", search_results.total_count);

    let latest_block_response = tendermint_client.latest_block_results().await?;
    let latest_block_height = latest_block_response.height.value();
    info!(
        "synchronizing blocks from {} to {}",
        initial_block_height, latest_block_height
    );

    let block_page_size = transaction_page_size;

    let mut current_height = initial_block_height;
    let mut last_log_height = 0;
    let key = "tx.height";
    while current_height < latest_block_height {
        let query = Query::gte(key, current_height).and_lt(key, current_height + block_page_size as u64);
        let search_results = tendermint_client.tx_search(
            query,
            false,
            1,
            transaction_page_size,
            tendermint_rpc::Order::Ascending
        ).await?;
        if search_results.total_count > 0 {
            println!("{} at height {}", search_results.total_count, current_height);
            for tx_response in search_results.txs.iter() {
                let mut events = BTreeMap::default();
                events.insert("tx.height".to_string(), vec![current_height.to_string()]);
                map_from_events(&tx_response.tx_result.events, &mut events)?;
                match Tx::from_bytes(tx_response.tx.as_bytes()) {
                    Ok(unmarshalled_tx) => {
                        if let Err(e) = process_parsed(registry, &unmarshalled_tx, &events, msg_set) {
                            error!("Error in process_parsed: {:?}", e);
                        }
                    }
                    Err(e) => {
                        error!("Error unmarshalling: {:?}", e);
                    }
                }
                // if let Err(e) = process_parsed(registry, &tx_response.tx, &events) {
                //     error!("Error in process_parsed: {:?}", e);
                // }
            }
            // println!("search_results: {:?}, txs:\n{:?}\ncurrent_height: {}", search_results.total_count, search_results.txs.iter().map(|tx| &tx.tx_result.log), current_height);
        }
        if current_height - last_log_height > 1000 {
            println!("current_height: {}", current_height);
            last_log_height = current_height;
        }
        current_height += block_page_size as u64;
    }

    // for block_height in initial_block_height..latest_block_height {
    //     if has_db {
    //         let db_block_opt = block
    //             .find(block_height as i64)
    //             .get_result::<Block>(registry.db.as_ref().unwrap())
    //             .optional()?;
    //         if db_block_opt.is_some() {
    //             return Ok(());
    //         }
    //     }

    //     if block_height % 1000 == 0 {
    //         info!("Added another 1000 blocks, height: {}", block_height);
    //     }

    //     let response = tendermint_client.block(block_height as u32).await?;
    //     let block_hash = response.block_id.hash.to_string();
    //     if save_all_blocks {
    //         let new_block = NewBlock::from_block_response(&block_hash, &response.block);
    //         if has_db {
    //             let db = registry.db.as_ref().unwrap();
    //             diesel::insert_into(block).values(&new_block).execute(db)?;
    //         }
    //     }

    //     // Look at the transactions:
    //     for tx in response.block.data.iter() {
    //         let tx_hash = tx_to_hash(tx);
    //         let tx_response = tendermint_client.tx(tx_hash, false).await?;
    //         let mut events = BTreeMap::default();
    //         events.insert("tx.height".to_string(), vec![block_height.to_string()]);
    //         map_from_events(&tx_response.tx_result.events, &mut events)?;
    //         match Tx::from_bytes(tx.as_bytes()) {
    //             Ok(unmarshalled_tx) => {
    //                 if let Err(e) = process_parsed(registry, &unmarshalled_tx, &events) {
    //                     error!("Error in process_parsed: {:?}", e);
    //                 }
    //             }
    //             Err(e) => {
    //                 error!("Error unmarshalling: {:?}", e);
    //             }
    //         }
    //     }
    // }
    Ok(())
}
