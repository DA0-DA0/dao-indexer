use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use anyhow::anyhow;
use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
use cosmrs::proto::cosmwasm::wasm::v1::{MsgExecuteContract, MsgInstantiateContract};
use cosmrs::tx::{MsgProto, Tx};
use diesel::PgConnection;
use log::error;
use prost_types::Any;
use tendermint_rpc::event::TxInfo;

pub fn process_parsed(
    conn: Option<&PgConnection>,
    registry: &IndexerRegistry,
    tx_parsed: &Tx,
    events: &EventMap,
) -> anyhow::Result<()> {
    process_messages(conn, registry, &tx_parsed.body.messages, events)
}

pub fn process_messages(
    conn: Option<&PgConnection>,
    registry: &IndexerRegistry,
    messages: &[Any],
    events: &EventMap,
) -> anyhow::Result<()> {
    for msg in messages.iter() {
        let type_url: &str = &msg.type_url;
        match type_url {
            "/cosmwasm.wasm.v1.MsgInstantiateContract" => {
                let msg_obj = MsgInstantiateContract::from_any(msg).map_err(|e| anyhow!(e))?;
                return msg_obj.index_message(conn, registry, events);
            }
            "/cosmwasm.wasm.v1.MsgExecuteContract" => {
                let msg_obj = MsgExecuteContract::from_any(msg).map_err(|e| anyhow!(e))?;
                // let msg_obj: MsgExecuteContract = MsgProto::from_any(msg)?;
                return msg_obj.index_message(conn, registry, events);
            }
            "/cosmos.bank.v1beta1.MsgSend" => {
                let msg_obj = MsgSend::from_any(msg).map_err(|e| anyhow!(e))?;
                // let msg_obj: MsgSend = MsgProto::from_any(msg)?;
                return msg_obj.index_message(conn, registry, events);
            }
            _ => {
                error!("No handler for {}", type_url);
            }
        }
    }
    Ok(())
}

pub fn process_tx_info(
    conn: Option<&PgConnection>,
    registry: &IndexerRegistry,
    tx_info: TxInfo,
    events: &EventMap,
) -> anyhow::Result<()> {
    let tx_parsed = Tx::from_bytes(&tx_info.tx).map_err(|e| anyhow!(e))?;
    process_parsed(conn, registry, &tx_parsed, events)
}
