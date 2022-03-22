use cosmrs::tx::{Raw};
use cosmrs::proto;
use tendermint::abci::Transaction;
use sha2::{Sha256, Digest};
use tendermint::abci::transaction::HASH_LENGTH;

pub fn tx_to_hash(tx: &Transaction) -> tendermint::abci::transaction::Hash {
    let rust_raw = Raw::from_bytes(tx.as_bytes()).unwrap();
    let tx_raw = proto::cosmos::tx::v1beta1::TxRaw::from(rust_raw);

    let mut tx_bytes = Vec::new();
    prost::Message::encode(&tx_raw, &mut tx_bytes).unwrap();
    let digest = Sha256::digest(&tx_bytes);
    let mut hash_bytes = [0u8; HASH_LENGTH];
    hash_bytes.copy_from_slice(&digest);

    tendermint::abci::transaction::Hash::new(hash_bytes)
}