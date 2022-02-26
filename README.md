
# Installing Diesel
dao-indexer uses the Diesel[https://diesel.rs/] ORM for Postgres. In addition
to the crates in Cargo.toml, you'll want to install the `diesel_cli` tool:

`cargo install diesel_cli --no-default-features --features postgres`

Note: this requires that the posgres libraries are on your link path already.

# Database Config
Copy `.env.example` to `.env` on your local system and edit the `DATABASE_URL` value to match your target postgres instance:

`DATABASE_URL=postgres://dbusername@localhost:5432/dbname`

