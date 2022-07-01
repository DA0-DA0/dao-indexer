use std::vec::Vec;

use anyhow::anyhow;
use diesel::prelude::*;
use tendermint_rpc::endpoint::tx::Response;

use crate::config::IndexerConfig;
use crate::db::models::{NewTransaction, Transaction};
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

        diesel::insert_into(transaction)
            .values(new_transaction)
            .execute(database_connection)?;
        Ok(())
    } else {
        Ok(())
    }
}


pub fn get_transactions(
    config: &IndexerConfig,
    indexer_registry: &IndexerRegistry,
) -> anyhow::Result<Vec<Response>> {
    if let Some(database_connection) = &indexer_registry.db {
        let txs = read_transaction(config, database_connection)?;
        let mut responses = Vec::new();
        for tx in txs {
            let parsed_response: Response = serde_json::from_value(tx.response)?;
            responses.push(parsed_response);
        }
        Ok(responses)
    } else {
        Err(anyhow!("Error: You need to define the database if you're trying to read from it."))
    }
}

fn read_transaction(
    config: &IndexerConfig,
    db_connection: &PgConnection
) -> anyhow::Result<Vec<Transaction>> {
    match transaction
        .filter(height.gt(config.tendermint_initial_block as i64))
        .filter(height.lt(config.tendermint_final_block as i64))
        .load::<Transaction>(db_connection) {
        Ok(_rows) => Ok(_rows),
        Err(e) => Err(anyhow!("Error: {:?}", e)),
    }
}