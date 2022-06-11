Prerequisites
(Install go, Install rust)

put $GOPATH/bin on your PATH. If you have homebrew and might have installed substreams, make sure $GOPATH/bin precedes the installed version. Commands below assume you're running the firehose and substreams executables that you have built locally.

```
git clone https://github.com/streamingfast/firehose-cosmos.git
cd firehose-cosmos
git checkout feature/substreams

```

This downloads some testnet blocks locally so firehose can serve them
```
export BLOCKS_DIR="./cosmos-blocks"
export FIREHOSE_API_TOKEN="1751e03cd2034d4b18f9365661875b49"
firehose-cosmos tools download-from-firehose firehose--cosmoshub-4--testnet.grpc.datahub.figment.io:443  --common-first-streamable-block=9034670 9100000 9101000 $BLOCKS_DIR
firehose-cosmos start firehose --common-first-streamable-block=9100000 --common-blockstream-addr= --common-blocks-store-url=$BLOCKS_DIR --substreams-enabled`
```

`curl https://get.wasmer.io/ -sSfL | sh`

^^  according to wasmer.io this is how you install wasmer library
(basically saying: "download anything from internet, and run it live"... secure)


`substreams run -p -e 127.0.0.1:9030 substream.yaml map_hello_world -s 9100300 -t +10`
