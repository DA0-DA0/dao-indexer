use anyhow::anyhow;
use diesel::prelude::*;
use tendermint_rpc::endpoint::tx::Response;

use crate::db::models::NewTransaction;
use crate::db::schema::transaction::dsl::*;
use crate::indexing::indexer_registry::IndexerRegistry;

pub fn insert_transaction(
    tx_response: &Response,
    indexer_registry: &IndexerRegistry,
) -> anyhow::Result<()> {
    if let Some(database_connection) = &indexer_registry.db {
        let hash_of_tx = tx_response.hash.to_string();
        let tx_as_json = serde_json::to_value(&tx_response)?;

        let new_transaction = NewTransaction {
            hash: hash_of_tx,
            height: tx_response.height.value() as i64,
            response: tx_as_json,
        };

        match diesel::insert_into(transaction)
            .values(new_transaction)
            .execute(database_connection)
        {
            Ok(_rows) => Ok(()),
            Err(e) => Err(anyhow!("Error: {:?}", e)),
        }
    } else {
        Ok(())
    }
}
