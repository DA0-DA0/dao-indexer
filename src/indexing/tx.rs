use super::event_map::EventMap;
use super::index_message::IndexMessage;
use super::indexer_registry::IndexerRegistry;
use anyhow::anyhow;
use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
use cosmrs::proto::cosmwasm::wasm::v1::{MsgExecuteContract, MsgInstantiateContract};
use cosmrs::tx::{MsgProto, Tx};
use log::error;
use prost_types::Any;
use std::collections::HashSet;
use tendermint_rpc::event::TxInfo;

pub fn process_parsed(
    registry: &IndexerRegistry,
    tx_parsed: &Tx,
    events: &EventMap,
    msg_set: &mut HashSet<String>
) -> anyhow::Result<()> {
    process_messages(registry, &tx_parsed.body.messages, events, msg_set)
}

pub fn process_messages(
    registry: &IndexerRegistry,
    messages: &[Any],
    events: &EventMap,
    msg_set: &mut HashSet<String>
) -> anyhow::Result<()> {
    for msg in messages.iter() {
        let type_url: &str = &msg.type_url;
        match type_url {
            "/cosmwasm.wasm.v1.MsgInstantiateContract" => {
                let msg_obj = MsgInstantiateContract::from_any(msg).map_err(|e| anyhow!(e))?;
                return msg_obj.index_message(registry, events);
            }
            "/cosmwasm.wasm.v1.MsgExecuteContract" => {
                let msg_obj = MsgExecuteContract::from_any(msg).map_err(|e| anyhow!(e))?;
                // let msg_obj: MsgExecuteContract = MsgProto::from_any(msg)?;
                return msg_obj.index_message(registry, events);
            }
            "/cosmos.bank.v1beta1.MsgSend" => {
                let msg_obj = MsgSend::from_any(msg).map_err(|e| anyhow!(e))?;
                // let msg_obj: MsgSend = MsgProto::from_any(msg)?;
                return msg_obj.index_message(registry, events);
            }
            // "/cosmos.staking.v1beta1.MsgDelegate" => {
            //     return Ok(());
            // }
            // "/cosmos.staking.v1beta1.MsgBeginRedelegate" => {
            //     return Ok(());
            // }
            // "/cosmos.staking.v1beta1.MsgWithdrawDelegatorReward" => {
            //     return Ok(());
            // }
            // "/cosmos.staking.v1beta1.MsgCreateValidator" => {
            //     return Ok(());
            // }
            // "/cosmos.staking.v1beta1.MsgWithdrawValidatorCommission" => {
            //     return Ok(());
            // }
            // "/cosmos.staking.v1beta1.MsgEditValidator" => {
            //     return Ok(());
            // }
            // "/cosmos.distribution.v1beta1.MsgWithdrawDelegatorReward" => {
            //     return Ok(());
            // }
            // "/cosmos.distribution.v1beta1.MsgWithdrawValidatorCommission" => {
            //     return Ok(());
            // }
            // "/cosmos.staking.v1beta1.MsgUndelegate" => {
            //     return Ok(());
            // }
            // "/ibc.core.client.v1.MsgCreateClient" => return Ok(()),
            // "/ibc.core.connection.v1.MsgConnectionOpenInit" => {
            //     return Ok(());
            // }
            // "/ibc.core.client.v1.MsgUpdateClient" => {
            //     return Ok(());
            // }
            // "/ibc.core.connection.v1.MsgConnectionOpenAck" => {
            //     return Ok(());
            // }
            // "/cosmos.slashing.v1beta1.MsgUnjail" => {
            //     return Ok(());
            // }
            // "/ibc.core.channel.v1.MsgChannelOpenInit" => {
            //     return Ok(());
            // }
            // "/ibc.core.channel.v1.MsgChannelOpenTry" => {
            //     return Ok(());
            // }
            // "/ibc.core.channel.v1.MsgRecvPacket" => {
            //     return Ok(());
            // }
            _ => {
                if !msg_set.contains(type_url) {
                    msg_set.insert(type_url.to_string());
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
    msg_set: &mut HashSet<String>
) -> anyhow::Result<()> {
    let tx_parsed = Tx::from_bytes(&tx_info.tx).map_err(|e| anyhow!(e))?;
    process_parsed(registry, &tx_parsed, events, msg_set)
}
