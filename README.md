
# CosmWasm Rust Indexer
A Rust application that queries the Tendermint APIs via both RPC and socket connections and updates a Postgres database with relevant information.

# PRE-ALPHA, DEVELOPERS ONLY PLEASE
This codebase is under heavy development. You're welcome to it, of course (it's MIT and all) but it's not a production solution yet. If you ARE a Rust developer interested in indexing cosmwasm/cosmos blockchain messages into a database, we welcome your pull requests. 

## Setup

### Database Setup

#### Install Diesel
dao-indexer uses the Diesel[https://diesel.rs/] ORM for Postgres. In addition
to the crates in Cargo.toml, you'll want to install the `diesel_cli` tool:

`cargo install diesel_cli --no-default-features --features postgres`

Note: this requires that the posgres libraries are on your link path already.

#### Database Config
Copy `.env.example` to `.env` on your local system and edit the `DATABASE_URL` value to match your target postgres instance:

`DATABASE_URL=postgres://dbusername@localhost:5432/dbname`

Example of what james has:

inside: example .env file
```
DATABASE_URL=postgres://james:MY_PASSWORD@localhost:5432/rustindexer
```

#### Running migrations
Below will run the actual migrations into your database and create tables.
```
diesel migration run
```

### Running Rust
Below will run the actual program.
```
cargo run
```

# Schema Indexer
A major project currently in progress uses the `JsonShema` trait all CosmWasm messages derive in order to automatically construct and populate various database tables for the contract messages. This is currently disabled by default as the code doesn't function yet. We hope to make the schema indexer the primary mechanism for mapping contract messages into database tables by the time of our production releases.

# Akash Deployment

## Prerequisites
- `akash` CLI tool installed locally
- funded wallet with sufficient `$AKT`

## Preparing a deployment
[[./akash.yaml]]
## Steps to deploy using CLI
This follows an adjusted https://docs.akash.network/guides/cli/streamlined-steps
0. Ensure all the images referenced by deployment are in the registries
1. Add keys to funded wallet to `akash` - can be checked with `akash keys list`
2. Generate one-time certificate for the keys using `akash tx cert`
```
akash tx cert generate client --from $AKASH_KEY_NAME
akash tx cert publish client --chain-id akashnet-2 --from $AKASH_WALLET --gas-prices="0.025uakt" --gas="auto" --gas-adjustment=1.15
```
3. Create deployment
```
akash tx --chain-id akashnet-2 --node "http://akash.c29r3.xyz:80/rpc" --from akash1rj4kqeayjz0cls95jj6jf8uzay7pnj4244zjaz  deployment create indexer-akash.yaml
```

4. Examine bids for hosting the workload
```
akash query --chain-id akashnet-2 --node "http://akash.c29r3.xyz:80/rpc" market bid list  > bids.txt
```

5. After identifying `provider` that meets needs, create a lease:
```
akash --chain-id akashnet-2 --dseq 1337 --node "http://akash.c29r3.xyz:80/rpc" tx market lease create --provider akash1vppf2922vuzgxc2jgetxtx7uxqfss59e7gha3g
```