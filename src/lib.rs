#[macro_use]
extern crate diesel;
extern crate dotenv;

pub mod db;
pub mod historical_parser;
pub mod indexing;
pub mod util;
pub mod config;
