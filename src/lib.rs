//! Index [`cosmwasm`]/[`Tendermint`] blockchain messages into SQL databases.
//!
//! The indexer project was started to provide indexing services for the
//! [`DA0-DA0`] DAO, and is slowly feature-creeping its way to being a generalized
//! toolkit for constructing application-specific indexes for any set of cosmwasm
//! transactions.
//!
//! [`Tendermint`]: https://tendermint.com/sdk/
//! [`cosmwasm`]: https://docs.cosmwasm.com/
//! [`DA0-DA0`]: https://daodao.zone

#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate env_logger;

/// Configure run-time indexing behavior
pub mod config;
/// Infrastructure for interacting with SQL databases
pub mod db;
/// Fetch and parse messages already committed to the blockchain
pub mod historical_parser;
/// Core indexing infrastructure
pub mod indexing;
/// Disorganized grab bag of utility functions used across the project.
pub mod util;
