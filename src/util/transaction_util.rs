use anyhow::anyhow;
use diesel::PgConnection;
use diesel::prelude::*;
use tendermint_rpc::endpoint::tx::Response;

use crate::db::models::NewTransaction;

use crate::db::schema::transaction::dsl::*;

pub fn insert_transaction(tx_response: &&Response, xt: &PgConnection) -> anyhow::Result<()>{
    let hash_of_tx = tx_response.hash.to_string();
    let tx_response_as_string = serde_json::to_string(&tx_response).unwrap();

    let new_transaction = NewTransaction {
        hash: hash_of_tx,
        height: tx_response.height.value() as i64,
        response: tx_response_as_string
    };

    match diesel::insert_into(transaction)
        .values(new_transaction)
        .execute(xt) {
        Ok(_) => { Ok(()) }
        Err(e) => { Err(anyhow!("Error: {:?}", e)) }
    }
}