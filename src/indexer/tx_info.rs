use super::index::Index;
use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
use cosmrs::proto::cosmwasm::wasm::v1::{MsgExecuteContract, MsgInstantiateContract};
use cosmrs::tx::{MsgProto, Tx};
use diesel::pg::PgConnection;
use std::collections::BTreeMap;
use tendermint_rpc::event::TxInfo;

pub fn process_tx_info(
  db: &PgConnection,
  tx_info: TxInfo,
  events: &Option<BTreeMap<String, Vec<String>>>,
) {
  match Tx::from_bytes(&tx_info.tx) {
    Ok(tx_parsed) => {
      for msg in tx_parsed.body.messages {
        let type_url: &str = &msg.type_url;
        match type_url {
          "/cosmwasm.wasm.v1.MsgInstantiateContract" => {
            let msg_obj: MsgInstantiateContract = MsgProto::from_any(&msg).unwrap();
            msg_obj.index(&db, &events);
          }
          "/cosmwasm.wasm.v1.MsgExecuteContract" => {
            let msg_obj: MsgExecuteContract = MsgProto::from_any(&msg).unwrap();
            msg_obj.index(&db, &events);
          }
          "/cosmos.bank.v1beta1.MsgSend" => {
            let msg_obj: MsgSend = MsgProto::from_any(&msg).unwrap();
            msg_obj.index(&db, &events);
          }
          _ => {
            eprintln!("No handler for {}", type_url);
          }
        }
      }
    }
    Err(err) => eprintln!("{:?}", err)
  }
}
