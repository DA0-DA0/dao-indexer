use super::index::Index;
use super::indexer_registry::IndexerRegistry;
use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
use cosmrs::proto::cosmwasm::wasm::v1::{MsgExecuteContract, MsgInstantiateContract};
use cosmrs::tx::{MsgProto, Tx};
use std::collections::BTreeMap;
use tendermint_rpc::event::TxInfo;
use prost_types::Any;


pub fn process_parsed(
  registry: &IndexerRegistry,
  tx_parsed: &Tx,
  events: &Option<BTreeMap<String, Vec<String>>>,
) -> Result<(), Box<dyn std::error::Error>> {
  process_messages(registry, &tx_parsed.body.messages, events)
}

pub fn process_messages(
  registry: &IndexerRegistry,
  messages: &[Any],
  events: &Option<BTreeMap<String, Vec<String>>>,
) -> Result<(), Box<dyn std::error::Error>> {

  for msg in messages.iter() {
    let type_url: &str = &msg.type_url;
    match type_url {
      "/cosmwasm.wasm.v1.MsgInstantiateContract" => {
        let msg_obj: MsgInstantiateContract = MsgProto::from_any(msg)?;
        return msg_obj.index(registry, events);
      }
      "/cosmwasm.wasm.v1.MsgExecuteContract" => {
        let msg_obj: MsgExecuteContract = MsgProto::from_any(msg)?;
        return msg_obj.index(registry, events);
      }
      "/cosmos.bank.v1beta1.MsgSend" => {
        let msg_obj: MsgSend = MsgProto::from_any(msg)?;
        return msg_obj.index(registry, events);
      }
      _ => {
        eprintln!("No handler for {}", type_url);
      }
    }
  }
  Ok(())
}

pub fn process_tx_info(
  registry: &IndexerRegistry,
  tx_info: TxInfo,
  events: &Option<BTreeMap<String, Vec<String>>>,
) -> Result<(), Box<dyn std::error::Error>> {
  let tx_parsed = Tx::from_bytes(&tx_info.tx)?;
  process_parsed(registry, &tx_parsed, events)
}
