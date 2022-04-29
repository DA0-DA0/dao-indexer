#!/bin/bash
LOCALHOST_RPC_URL=http://localhost:26657
AWS_RPC_URL=http://54.177.5.188:26657
NEXT_PUBLIC_CHAIN_RPC_ENDPOINT=https://rpc-juno.itastakers.com:443
NEXT_PUBLIC_CHAIN_REST_ENDPOINT=https://lcd-juno.itastakers.com:443
TESTNET_RPC_ENDPOINT=https://rpc.uni.juno.deuslabs.fi

MAINNET_BLOCK_START=1056100
TENDERMINT_INITIAL_BLOCK_HEIGHT=$MAINNET_BLOCK_START
PAGE=1
PER_PAGE=100

TENDERMINT_RPC_URL=$TESTNET_RPC_ENDPOINT
# so the number of transactions is determined by the TENDERMINT_INITIAL_BLOCK_HEIGHT + PER_PAGE
CMD="${TENDERMINT_RPC_URL}/tx_search?query=\"tx.height>${TENDERMINT_INITIAL_BLOCK_HEIGHT}\"&prove=false&page=${PAGE}&per_page=${PER_PAGE}&order_by=\"asc\""
curl $CMD
