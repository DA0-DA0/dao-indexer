use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use anyhow::anyhow;
use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
use cosmrs::proto::cosmwasm::wasm::v1::{MsgExecuteContract, MsgInstantiateContract};
use cosmrs::tx::{MsgProto, Tx};
use log::{debug, error};
use prost_types::Any;
// use std::borrow::BorrowMut;
use std::collections::HashSet;
use std::sync::Arc;
use tendermint_rpc::event::TxInfo;

pub fn process_parsed(
    registry: &IndexerRegistry,
    tx_parsed: &Tx,
    events: &EventMap,
    msg_set: Arc<HashSet<String>>,
) -> anyhow::Result<()> {
    process_messages(registry, &tx_parsed.body.messages, events, msg_set)
}

pub fn process_parsed_v1beta(
    registry: &IndexerRegistry,
    tx_parsed: &cosmos_sdk_proto::cosmos::tx::v1beta1::Tx,
    events: &EventMap,
    msg_set: Arc<HashSet<String>>,
) -> anyhow::Result<()> {
    if let Some(body) = &tx_parsed.body {
        process_messages(registry, &body.messages, events, msg_set)
    } else {
        Ok(())
    }
}

pub fn process_messages(
    registry: &IndexerRegistry,
    messages: &[Any],
    events: &EventMap,
    msg_set: Arc<HashSet<String>>,
) -> anyhow::Result<()> {
    for msg in messages.iter() {
        let type_url: &str = &msg.type_url;
        debug!("processing msg {:?}", msg);
        match type_url {
            "/cosmwasm.wasm.v1.MsgInstantiateContract" => {
                match MsgInstantiateContract::from_any(msg) {
                    Ok(msg_obj) => {
                        return msg_obj.index_message(registry, events);
                    }
                    Err(e) => {
                        error!("error parsing MsgInstantiateContract, events: {:?}", events);
                        return Err(anyhow!(e));
                    }
                }
            }
            "/cosmwasm.wasm.v1.MsgExecuteContract" => match MsgExecuteContract::from_any(msg) {
                Ok(msg_obj) => {
                    return msg_obj.index_message(registry, events);
                }
                Err(e) => {
                    error!("error parsing MsgExecuteContract, events: {:?}", events);
                    return Err(anyhow!(e));
                }
            },
            "/cosmos.bank.v1beta1.MsgSend" => match MsgSend::from_any(msg) {
                Ok(msg_obj) => {
                    return msg_obj.index_message(registry, events);
                }
                Err(e) => {
                    error!("error parsing MsgSend, events: {:?}", events);
                    return Err(anyhow!(e));
                }
            },
            _ => {
                if !msg_set.contains(type_url) {
                    // msg_set.borrow_mut().insert(type_url.to_string());
                    error!("No handler for {}", type_url);
                }
            }
        }
    }
    Ok(())
}

pub fn process_tx_info(
    registry: &IndexerRegistry,
    tx_info: TxInfo,
    events: &EventMap,
    msg_set: Arc<HashSet<String>>,
) -> anyhow::Result<()> {
    let tx_parsed = Tx::from_bytes(&tx_info.tx).map_err(|e| anyhow!(e))?;
    process_parsed(registry, &tx_parsed, events, msg_set)
}
