use crate::config::IndexerConfig;
use crate::indexing::event_map::EventMap;
use crate::indexing::indexer_registry::IndexerRegistry;
use crate::indexing::msg_set::MsgSet;
use crate::indexing::tx::{process_parsed, process_parsed_v1beta};
use crate::util::query_stream::{QueryStream, TxSearchRequest};
use cosmos_sdk_proto::cosmos::tx::v1beta1::Tx as TxV1;
use cosmrs::tx::Tx;
use futures::future::join_all;
use futures::FutureExt;
use log::{debug, error, info, warn};
use math::round;
use prost::Message;
use std::cmp::min;
use std::collections::BTreeMap;
use tendermint::abci::responses::Event;
use tendermint_rpc::endpoint::tx_search::Response as TxSearchResponse;
use tendermint_rpc::query::Query;
use tendermint_rpc::Client;
use tendermint_rpc::HttpClient as TendermintClient;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

// This is a tech debut function that maps events into a structure
// that's a little easier to index.
fn map_from_events(events: &[Event], event_map: &mut EventMap) -> anyhow::Result<()> {
    for event in events {
        let event_name = &event.type_str;
        for attribute in &event.attributes {
            let attribute_key: &str = &attribute.key.to_string();
            let event_key = format!("{}.{}", event_name, attribute_key);
            let attributes = match event_map.get_mut(&event_key) {
                Some(existing_attributes) => existing_attributes,
                _ => {
                    event_map.insert(event_key.clone(), vec![]);
                    event_map
                        .get_mut(&event_key)
                        .ok_or_else(|| anyhow::anyhow!("no attribute {} found", event_key))?
                }
            };
            attributes.push(attribute.value.to_string());
        }
    }
    Ok(())
}

// Generic driver function for "put these blockchain transactions into the index".
// This function isn't actually async, but may eventually be
// extended to code that is, hence it being marked that way.
async fn index_search_results(
    search_results: TxSearchResponse,
    registry: &IndexerRegistry,
    msg_set: MsgSet,
) -> anyhow::Result<()> {
    if search_results.total_count < 1 {
        return Ok(());
    }
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
                    error!("Error in process_parsed: {:?}\n{:?}", e, unmarshalled_tx);
                }
            }
            Err(e) => {
                warn!(
                    "Error unmarshalling: {:?} via Tx::from_bytes, trying v1beta decode",
                    e
                );
                info!("tx_response:\n{:?}", tx_response);
                match TxV1::decode(tx_response.tx.as_bytes()) {
                    // match TxV1::decode(tx_response.tx.as_bytes()) {
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

async fn requeue(
    config: &IndexerConfig,
    queries_mutex: &Mutex<QueryStream>,
    tx_request: Box<TxSearchRequest>,
) -> anyhow::Result<()> {
    if config.requeue_sleep > 0 {
        std::thread::sleep(std::time::Duration::from_millis(config.requeue_sleep));
    }
    debug!("Requeing tx_request {:?}", &tx_request);
    let mut queries = queries_mutex.lock().await;
    queries.enqueue(tx_request);
    //  {
    //     Ok(mut queries) => {
    //         queries.enqueue(tx_request);
    //     }
    //     Err(e) => {
    //         error!("Error unlocking queries mutex: {:?}", e);
    //     }
    // }
    Ok(())
}

// Process the response from calling an RPC. This can result in additional
// RPC calls being queued.
#[allow(clippy::too_many_arguments)]
async fn handle_transaction_response(
    response: Result<TxSearchResponse, tendermint_rpc::error::Error>,
    tx_request: Box<TxSearchRequest>,
    current_height: u64,
    last_block: u64,
    registry: &IndexerRegistry,
    config: &IndexerConfig,
    msg_set: MsgSet,
    queries_mutex: &Mutex<QueryStream>,
) -> anyhow::Result<()> {
    match response {
        Ok(search_results) => {
            let total_count = search_results.total_count;
            // Retry logic for empty page 1 blocks:
            if total_count == 0 && tx_request.page == 1 {
                // There was no error, and no results. Look again.
                if tx_request.reque_count < config.max_empty_block_retries as i64 {
                    return requeue(config, queries_mutex, tx_request).await;
                } else {
                    warn!(
                        "Received empty results for request {:#?}/{:#?} after {} retries",
                        &tx_request, &tx_request.query, tx_request.reque_count
                    );
                    return Ok(());
                }
            }
            let total_pages =
                round::ceil(total_count as f64 / config.transaction_page_size as f64, 0) as u32;
            if total_count > 0 && tx_request.page == 1 {
                info!(
                    "received {} for blocks {}-{} (requeue: {}); at {} items/page, {} total pages",
                    search_results.total_count,
                    current_height,
                    last_block,
                    tx_request.reque_count,
                    config.transaction_page_size,
                    total_pages
                );
            } else {
                debug!(
                    "received page {}, blocks {}-{} (requeue: {}): {} items",
                    tx_request.page,
                    current_height,
                    last_block,
                    tx_request.reque_count,
                    search_results.txs.len()
                );
            }
            if total_pages > 1 && tx_request.page == 1 {
                let mut queries = queries_mutex.lock().await;
                for page in 2..=total_pages {
                    // Inclusive range
                    let tx_search =
                        TxSearchRequest::from_query_and_page(tx_request.query.clone(), page);
                    debug!(
                        "enqueing query for page {}, blocks {}-{}",
                        page, current_height, last_block
                    );
                    queries.enqueue(Box::new(tx_search));
                }
            }
            index_search_results(search_results, registry, msg_set.clone()).await?;
        }
        Err(e) => {
            debug!(
                "Error: {:?}\npage: {}, current_height:{}",
                e, tx_request.page, current_height
            );
            return requeue(config, queries_mutex, tx_request).await;
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
    info!(
        "loading transactions for blocks {}-{}",
        current_height, last_block
    );
    let key = "tx.height";
    let query = Query::gte(key, current_height).and_lt(key, last_block + 1);
    let page_one_request = TxSearchRequest::from_query_and_page(query, 1);
    let mut queries = QueryStream::new();
    queries.enqueue(Box::new(page_one_request));
    let queries_mutex = Mutex::from(queries);
    let mut page_futures = vec![];
    let max_requests = config.max_requests as usize;
    loop {
        let mut query = None;
        let mut page = 0;
        let mut tx_request = None;
        {
            let mut queries_stream = queries_mutex.lock().await;
            if let Some(next_tx_request) = queries_stream.next().await {
                query = Some(next_tx_request.query.clone());
                page = next_tx_request.page;
                tx_request = Some(next_tx_request);
            }
        }
        if query.is_some() && tx_request.is_some() {
            let query = query.unwrap();
            let tx_request = tx_request.unwrap();
            let requeue_count = tx_request.reque_count;
            if requeue_count > 0 {
                debug!(
                    "Attempting re-queued tx_request for page {} after {} re-queues",
                    tx_request.page, tx_request.reque_count
                );
            }
            let f = tendermint_client
                .tx_search(
                    query.clone(),
                    false,
                    page,
                    config.transaction_page_size,
                    tendermint_rpc::Order::Ascending,
                )
                .map(|response| {
                    handle_transaction_response(
                        response,
                        tx_request,
                        current_height,
                        last_block,
                        registry,
                        config,
                        msg_set.clone(),
                        &queries_mutex,
                    )
                });
            page_futures.push(f);
            if page_futures.len() == max_requests || requeue_count > 0 {
                let results_futures = join_all(page_futures).await;
                join_all(results_futures).await;
                page_futures = vec![];
            }
        } else {
            if page_futures.is_empty() {
                break;
            }
            let results_futures = join_all(page_futures).await;
            if results_futures.is_empty() {
                break;
            }
            join_all(results_futures).await;
            page_futures = vec![];
        }
    }
    Ok(())
}

pub async fn block_synchronizer(
    registry: &IndexerRegistry,
    config: &IndexerConfig,
    msg_set: MsgSet,
) -> anyhow::Result<()> {
    info!("Loading RPC client for {}", &config.tendermint_rpc_url);
    let tendermint_client = TendermintClient::new::<&str>(&config.tendermint_rpc_url)?;
    info!("Waiting for healthy RPC node...");
    tendermint_client
        .wait_until_healthy(std::time::Duration::from_millis(1000))
        .await?;
    info!("RPC node is healthy, starting historical indexing");
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
    let max_requests = config.max_requests;
    let mut outstanding_requests = 0;
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
            info!(
                "queued blocks at heights {}-{}",
                last_log_height, current_height
            );
            last_log_height = current_height;
        }
        let remaining: i64 =
            latest_block_height as i64 - current_height as i64 - config.block_page_size as i64;
        if remaining <= 0 {
            current_height = latest_block_height; // break out of the loop
        } else if remaining < config.block_page_size as i64 {
            current_height += remaining as u64;
        } else {
            current_height += config.block_page_size as u64;
        }
        outstanding_requests += 1;
        if outstanding_requests == max_requests {
            join_all(block_transaction_futures).await;
            outstanding_requests = 0;
            block_transaction_futures = vec![];
        }
    }
    join_all(block_transaction_futures).await;
    Ok(())
}
