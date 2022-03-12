-- Your SQL goes here
CREATE TABLE block
(
    height           BIGINT UNIQUE PRIMARY KEY,
    hash             TEXT                        NOT NULL UNIQUE,
    num_txs          BIGINT DEFAULT 0
);