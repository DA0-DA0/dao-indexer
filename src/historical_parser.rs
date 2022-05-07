use crate::config::IndexerConfig;
use crate::indexing::event_map::EventMap;
use crate::indexing::indexer_registry::IndexerRegistry;
use crate::indexing::msg_set::MsgSet;
use crate::indexing::tx::{process_parsed, process_parsed_v1beta};
use crate::util::query_stream::{QueryStream, TxSearchRequest};
use async_std::stream::StreamExt;
use cosmrs::tx::Tx;
use futures::future::join_all;
use futures::FutureExt;
use log::{debug, error, info, warn};
use math::round;
use prost::Message;
use std::cmp::min;
use std::collections::BTreeMap;
use std::sync::Mutex;
use tendermint::abci::responses::Event;
use tendermint_rpc::endpoint::tx_search::Response as TxSearchResponse;
use tendermint_rpc::query::Query;
use tendermint_rpc::Client;
use tendermint_rpc::HttpClient as TendermintClient;

fn map_from_events(events: &[Event], event_map: &mut EventMap) -> anyhow::Result<()> {
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

async fn index_search_results(
    search_results: TxSearchResponse,
    registry: &IndexerRegistry,
    msg_set: MsgSet,
) -> anyhow::Result<()> {
    info!(
        "index_search_results txs: {} total_count: {}",
        search_results.txs.len(),
        search_results.total_count
    );
    for tx_response in search_results.txs.iter() {
        let msg_set = msg_set.clone();
        let mut events = BTreeMap::default();
        let block_height = tx_response.height;
        map_from_events(&tx_response.tx_result.events, &mut events)?;
        if events.get("tx.height").is_none() {
            events.insert("tx.height".to_string(), vec![block_height.to_string()]);
        }
        match Tx::from_bytes(tx_response.tx.as_bytes()) {
            Ok(unmarshalled_tx) => {
                if let Err(e) = process_parsed(registry, &unmarshalled_tx, &events, msg_set) {
                    error!("Error in process_parsed: {:?}", e);
                }
            }
            Err(e) => {
                warn!(
                    "Error unmarshalling: {:?} via Tx::from_bytes, trying v1beta decode",
                    e
                );
                info!("tx_response:\n{:?}", tx_response);
                match cosmos_sdk_proto::cosmos::tx::v1beta1::Tx::decode(tx_response.tx.as_bytes()) {
                    Ok(unmarshalled_tx) => {
                        info!("decoded response debug:\n{:?}", unmarshalled_tx);
                        if let Err(e) =
                            process_parsed_v1beta(registry, &unmarshalled_tx, &events, msg_set)
                        {
                            error!("Error in process_parsed: {:?}", e);
                        }
                    }
                    Err(e) => {
                        error!("Error decoding: {:?}", e);
                    }
                }
            }
        }
    }
    Ok(())
}

pub async fn load_block_transactions(
    tendermint_client: &TendermintClient,
    config: &IndexerConfig,
    registry: &IndexerRegistry,
    msg_set: MsgSet,
    current_height: u64,
) -> anyhow::Result<()> {
    let mut last_block = current_height + config.block_page_size;
    if config.tendermint_final_block > 0 {
        last_block = min(last_block, config.tendermint_final_block);
    }
    debug!("loading blocks {}-{}", current_height, last_block);
    let key = "tx.height";
    let query = Query::gte(key, current_height).and_lt(key, last_block + 1);
    let page_one_request = TxSearchRequest::from_query_and_page(query, 1);
    let mut queries = QueryStream::new();
    queries.enqueue(Box::new(page_one_request));
    let queries = Mutex::from(queries);
    process_queries(
        tendermint_client,
        config,
        registry,
        msg_set.clone(),
        current_height,
        last_block,
        queries,
    )
    .await
}

pub async fn process_queries(
    tendermint_client: &TendermintClient,
    config: &IndexerConfig,
    registry: &IndexerRegistry,
    msg_set: MsgSet,
    current_height: u64,
    last_block: u64,
    queries: Mutex<QueryStream>,
) -> anyhow::Result<()> {
    let mut page_futures = vec![];
    loop {
        let query;
        let page;
        let tx_request;
        if let Some(next_tx_request) = queries.lock().unwrap().next().await {
            tx_request = next_tx_request;
            query = tx_request.query.clone();
            page = tx_request.page;
        } else {
            break;
        }
        let f = tendermint_client
            .tx_search(
                query.clone(),
                false,
                page,
                config.transaction_page_size,
                tendermint_rpc::Order::Ascending,
            )
            .map(|response| match response {
                Ok(search_results) => {
                    let total_count = search_results.total_count;
                    // if total_count == 0 {
                    //     return Ok(());
                    // }
                    let total_pages =
                        round::ceil(total_count as f64 / config.transaction_page_size as f64, 0)
                            as u32;
                    info!(
                        "received {} for block {}, at {} items per page this is {} total pages",
                        search_results.total_count,
                        current_height,
                        config.transaction_page_size,
                        total_pages
                    );
                    info!(
                        "indexing page {}, blocks {}-{}",
                        tx_request.page, current_height, last_block
                    );
                    // Iterate through all the pages in the results:
                    if total_pages > 1 && tx_request.page == 1 {
                        for page in 2..=total_pages {
                            // Inclusive range
                            let tx_search = TxSearchRequest::from_query_and_page(
                                tx_request.query.clone(),
                                page,
                            );
                            info!(
                                "enqueing query for page {}, blocks {}-{}",
                                page, current_height, last_block
                            );
                            queries.lock().unwrap().enqueue(Box::new(tx_search));
                        }
                    }
                    index_search_results(search_results, registry, msg_set.clone())
                }
                Err(e) => {
                    error!("{:?}\nRequeing tx_request", e);
                    queries.lock().unwrap().enqueue(tx_request);
                    let empty_response = TxSearchResponse {
                        txs: vec![],
                        total_count: 0,
                    };
                    index_search_results(empty_response, registry, msg_set.clone())
                }
            });
        page_futures.push(f);
    }
    let _ = join_all(page_futures).await;
    //     {
    //         Ok(search_results) => {
    //             handle_search_results(
    //                 config,
    //                 tendermint_client,
    //                 tx_request,
    //                 queries,
    //                 search_results,
    //                 registry,
    //                 msg_set.clone(),
    //                 current_height,
    //                 last_block,
    //             )
    //             .await?
    //         }
    //         Err(e) => {
    //             error!("{:?}, requeuing...", e);
    //             queries.enqueue(tx_request);
    //             return Ok(());
    //         }
    //     }
    // }
    Ok(())
}

// #[allow(clippy::too_many_arguments)]
// async fn handle_search_results(
//     config: &IndexerConfig,
//     tendermint_client: &TendermintClient,
//     tx_request: Box<TxSearchRequest>,
//     queries: &mut QueryStream,
//     search_results: TxSearchResponse,
//     registry: &IndexerRegistry,
//     msg_set: MsgSet,
//     current_height: u64,
//     last_block: u64,
// ) -> anyhow::Result<()> {
//     let total_count = search_results.total_count;
//     if total_count == 0 {
//         return Ok(());
//     }
//     let total_pages =
//         round::ceil(total_count as f64 / config.transaction_page_size as f64, 0) as u32;
//     info!(
//         "received {} for block {}, at {} items per page this is {} total pages",
//         search_results.total_count, current_height, config.transaction_page_size, total_pages
//     );
//     info!("indexing page {}, blocks {}-{}", tx_request.page, current_height, last_block);
//     index_search_results(search_results, registry, msg_set.clone()).await?;

//     // Iterate through all the pages in the results:
//     if total_pages > 1 {
//         for page in 2..=total_pages {
//             // Inclusive range
//             let async_query = query.clone();
//             let tx_search = TxSearchRequest::from_query_and_page(tx_request.query.clone(), page);
//             queries.enqueue(Box::new(tx_search));

//             let msg_set = msg_set.clone();
//             info!(
//                 "querying for page {}, blocks {}-{}",
//                 page, current_height, last_block
//             );
//             let f = tendermint_client
//                 .tx_search(
//                     async_query,
//                     false,
//                     page,
//                     config.transaction_page_size,
//                     tendermint_rpc::Order::Ascending,
//                 )
//                 .map(|response| match response {
//                     Ok(search_results) => index_search_results(search_results, registry, msg_set),
//                     Err(e) => {
//                         error!("{:?}", e);
//                         let empty_response = TxSearchResponse {
//                             txs: vec![],
//                             total_count: 0,
//                         };
//                         index_search_results(empty_response, registry, msg_set)
//                     }
//                 });
//             page_futures.push(f);
//         }
//     }
//     info!("wating for {} total_pages...", total_pages);
//     let _ = join_all(page_futures).await;
//     info!("received {} total_pages", total_pages);
//     Ok(())
// }

pub async fn block_synchronizer(
    registry: &IndexerRegistry,
    config: &IndexerConfig,
    msg_set: MsgSet,
) -> anyhow::Result<()> {
    let tendermint_client = TendermintClient::new::<&str>(&config.tendermint_rpc_url)?;
    let mut latest_block_height = config.tendermint_final_block;
    if config.tendermint_final_block == 0 {
        let latest_block_response = tendermint_client.latest_block_results().await?;
        latest_block_height = latest_block_response.height.value();
    }
    info!(
        "synchronizing blocks from {} to {}",
        config.tendermint_initial_block, latest_block_height
    );
    if latest_block_height < config.tendermint_initial_block {
        error!(
            "Requested start at {} but latest block height is {}",
            config.tendermint_initial_block, latest_block_height
        );
        return Ok(());
    }

    let mut current_height = config.tendermint_initial_block;
    let mut last_log_height = config.tendermint_initial_block;
    let mut block_transaction_futures = vec![];
    while current_height < latest_block_height {
        // run load_block_transactions calls in in parallel
        let f = load_block_transactions(
            &tendermint_client,
            config,
            registry,
            msg_set.clone(),
            current_height,
        );
        block_transaction_futures.push(f);
        if current_height - last_log_height > 1000 {
            info!("indexed heights {}-{}", last_log_height, current_height);
            last_log_height = current_height;
        }
        current_height += config.block_page_size as u64;
    }
    join_all(block_transaction_futures).await;
    Ok(())
}
