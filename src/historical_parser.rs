use crate::indexing::indexer_registry::IndexerRegistry;
use crate::indexing::tx::{process_parsed, process_parsed_v1beta};
use cosmrs::tx::Tx;
use futures::future::join_all;
// use futures::prelude::*;
use crate::config::IndexerConfig;
use futures::FutureExt;
use log::{error, info, warn};
use math::round;
use prost::Message;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::sync::Arc;
use tendermint::abci::responses::Event;
use tendermint_rpc::endpoint::tx_search::Response as TxSearchResponse;
use tendermint_rpc::query::Query;
use tendermint_rpc::Client;
use tendermint_rpc::HttpClient as TendermintClient;

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

async fn index_search_results(
    search_results: TxSearchResponse,
    registry: &IndexerRegistry,
    msg_set: Arc<HashSet<String>>,
) -> anyhow::Result<()> {
    info!(
        "index_search_results txs: {} total_count: {}",
        search_results.txs.len(),
        search_results.total_count
    );
    for tx_response in search_results.txs.iter() {
        let mut events = BTreeMap::default();
        let block_height = tx_response.height;
        map_from_events(&tx_response.tx_result.events, &mut events)?;
        if events.get("tx.height").is_none() {
            events.insert("tx.height".to_string(), vec![block_height.to_string()]);
            // info!("created tx.height of {}", block_height);
        }
        match Tx::from_bytes(tx_response.tx.as_bytes()) {
            Ok(unmarshalled_tx) => {
                if let Err(e) = process_parsed(registry, &unmarshalled_tx, &events, msg_set.clone())
                {
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
                        if let Err(e) = process_parsed_v1beta(
                            registry,
                            &unmarshalled_tx,
                            &events,
                            msg_set.clone(),
                        ) {
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
    transaction_page_size: u8,
    registry: &IndexerRegistry,
    msg_set: Arc<HashSet<String>>,
    current_height: u64,
    block_page_size: u64,
) -> anyhow::Result<()> {
    let last_block = current_height + block_page_size;
    info!("loading blocks {}-{}", current_height, last_block - 1);
    let key = "tx.height";
    let query = Query::gte(key, current_height).and_lt(key, last_block);
    let search_results = tendermint_client
        .tx_search(
            query.clone(),
            false,
            1,
            transaction_page_size,
            tendermint_rpc::Order::Ascending,
        )
        .await?;
    let total_pages = round::ceil(
        search_results.total_count as f64 / transaction_page_size as f64,
        0,
    ) as u32;
    info!(
        "received {} for block {}, at {} items per page this is {} total pages",
        search_results.total_count, current_height, transaction_page_size, total_pages
    );
    info!(
        "indexing page 1, blocks {}-{}",
        current_height,
        last_block - 1
    );
    index_search_results(search_results, registry, msg_set.clone()).await?;

    // Iterate through all the pages in the results:
    let mut page_futures = vec![];
    if total_pages > 1 {
        for page in 2..=total_pages {
            // Inclusive range
            let async_query = query.clone();
            info!(
                "querying for page {}, blocks {}-{}",
                page,
                current_height,
                last_block - 1
            );
            let f = tendermint_client
                .tx_search(
                    async_query,
                    false,
                    page,
                    transaction_page_size,
                    tendermint_rpc::Order::Ascending,
                )
                .map(|response| match response {
                    Ok(search_results) => {
                        index_search_results(search_results, registry, msg_set.clone())
                    }
                    Err(e) => {
                        error!("{:?}", e);
                        let empty_response = TxSearchResponse {
                            txs: vec![],
                            total_count: 0,
                        };
                        index_search_results(empty_response, registry, msg_set.clone())
                    }
                });
            page_futures.push(f);
        }
    }
    info!("wating for {} total_pages...", total_pages);
    let _ = join_all(page_futures).await;
    info!("received {} total_pages", total_pages);
    Ok(())
}

pub async fn block_synchronizer(
    registry: &IndexerRegistry,
    config: &IndexerConfig,
    // tendermint_rpc_url: &str,
    // initial_block_height: u64,
    // tendermint_final_block: u64,
    // _save_all_blocks: bool,
    // transaction_page_size: u8,
    // block_page_size: u64,
    msg_set: Arc<HashSet<String>>,
) -> anyhow::Result<()> {
    let tendermint_client = TendermintClient::new::<&str>(&config.tendermint_rpc_url)?;
    let latest_block_response = tendermint_client.latest_block_results().await?;
    let mut latest_block_height = latest_block_response.height.value();
    if config.tendermint_final_block != 0 {
        latest_block_height = config.tendermint_final_block;
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
    let mut last_log_height = 0;
    while current_height < latest_block_height {
        // TODO(gavin.doughtie): we should be able to run N of these
        // load_block_transactions calls in a loop and have them run
        // in parallel!
        load_block_transactions(
            &tendermint_client,
            config.transaction_page_size,
            registry,
            msg_set.clone(),
            current_height,
            config.block_page_size,
        )
        .await?;
        if current_height - last_log_height > 1000 {
            info!("indexed heights {}-{}", last_log_height, current_height);
            last_log_height = current_height;
        }
        current_height += config.block_page_size as u64;
    }
    Ok(())
}

pub fn init_known_unknown_messages(msg_set: &mut HashSet<String>) {
    let known = [
        "/cosmos.authz.v1beta1.MsgExec",
        "/cosmos.authz.v1beta1.MsgGrant",
        "/cosmos.distribution.v1beta1.MsgSetWithdrawAddress",
        "/cosmos.distribution.v1beta1.MsgWithdrawDelegatorReward",
        "/cosmos.distribution.v1beta1.MsgWithdrawValidatorCommission",
        "/cosmos.feegrant.v1beta1.MsgGrantAllowance",
        "/cosmos.feegrant.v1beta1.MsgRevokeAllowance",
        "/cosmos.gov.v1beta1.MsgVote",
        "/cosmos.slashing.v1beta1.MsgUnjail",
        "/cosmos.staking.v1beta1.MsgBeginRedelegate",
        "/cosmos.staking.v1beta1.MsgCreateValidator",
        "/cosmos.staking.v1beta1.MsgDelegate",
        "/cosmos.staking.v1beta1.MsgEditValidator",
        "/cosmos.staking.v1beta1.MsgUndelegate",
        "/cosmos.staking.v1beta1.MsgWithdrawDelegatorReward",
        "/cosmos.staking.v1beta1.MsgWithdrawValidatorCommission",
        "/cosmwasm.wasm.v1.MsgStoreCode",
        "/ibc.applications.transfer.v1.MsgTransfer",
        "/ibc.core.channel.v1.MsgAcknowledgement",
        "/ibc.core.channel.v1.MsgChannelOpenInit",
        "/ibc.core.channel.v1.MsgChannelOpenTry",
        "/ibc.core.channel.v1.MsgRecvPacket",
        "/ibc.core.channel.v1.MsgTimeout",
        "/ibc.core.client.v1.MsgCreateClient",
        "/ibc.core.client.v1.MsgUpdateClient",
        "/ibc.core.connection.v1.MsgConnectionOpenAck",
        "/ibc.core.connection.v1.MsgConnectionOpenInit",
    ];
    known.map(|msg| msg_set.insert(msg.to_string()));
}
