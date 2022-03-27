use cosmrs::proto;
use cosmrs::tx::Raw;
use sha2::{Digest, Sha256};
use tendermint::abci::transaction::HASH_LENGTH;
use tendermint::abci::Transaction;

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
