use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use super::msg_set::MsgSet;
use anyhow::anyhow;
use cosmrs::cosmwasm::MsgInstantiateContract;
use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
use cosmrs::proto::cosmwasm::wasm::v1::{
    MsgExecuteContract, MsgInstantiateContract as ProtoMsgInstContrct,
};
use cosmrs::tx::{MsgProto, Tx};
use log::{debug, error};
use prost_types::Any;
use tendermint_rpc::event::TxInfo;

pub fn process_parsed(
    registry: &IndexerRegistry,
    tx_parsed: &Tx,
    events: &EventMap,
    msg_set: MsgSet,
) -> anyhow::Result<()> {
    process_messages(registry, &tx_parsed.body.messages, events, msg_set)
}

pub fn process_parsed_v1beta(
    registry: &IndexerRegistry,
    tx_parsed: &cosmos_sdk_proto::cosmos::tx::v1beta1::Tx,
    events: &EventMap,
    msg_set: MsgSet,
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
    msg_set: MsgSet,
) -> anyhow::Result<()> {
    for msg in messages.iter() {
        let type_url: &str = &msg.type_url;
        debug!("processing msg {:?}", msg);
        match type_url {
            "/cosmwasm.wasm.v1.MsgInstantiateContract" => {
                match ProtoMsgInstContrct::from_any(msg) {
                    Ok(proto_msg_instantiate_contract) => {
                        match MsgInstantiateContract::try_from(proto_msg_instantiate_contract) {
                            Ok(msg_inst_contract) => {
                                return msg_inst_contract.index_message(registry, events);
                            }
                            Err(e) => {
                                error!(
                                    "error parsing MsgInstantiateContract, events: {:?}",
                                    events
                                );
                                return Err(anyhow!(e));
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            "error parsing ProstMsgInstantiateContract, events: {:?}",
                            events
                        );
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
                let mut msg_set_writable = msg_set.lock().unwrap();
                let found = msg_set_writable.validate(type_url);
                if !found {
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
    msg_set: MsgSet,
) -> anyhow::Result<()> {
    let tx_parsed = Tx::from_bytes(&tx_info.tx).map_err(|e| anyhow!(e))?;
    process_parsed(registry, &tx_parsed, events, msg_set)
}
