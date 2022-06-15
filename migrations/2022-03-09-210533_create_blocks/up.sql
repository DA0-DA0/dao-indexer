CREATE TABLE block (
    height BIGINT UNIQUE PRIMARY KEY,
    hash TEXT NOT NULL UNIQUE,
    num_txs BIGINT DEFAULT 0
);

CREATE TABLE transaction (
    hash TEXT UNIQUE PRIMARY KEY,
    height BIGINT,
    response TEXT
);