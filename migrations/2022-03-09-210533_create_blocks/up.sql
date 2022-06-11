CREATE TABLE block (
    height BIGINT UNIQUE PRIMARY KEY,
    hash TEXT NOT NULL UNIQUE,
    num_txs BIGINT DEFAULT 0
);

-- events are in events, map of sequences
-- messages are in events, map of sequences
CREATE TABLE tx (
    hash TEXT UNIQUE PRIMARY KEY
    height BIGINT
    messages BYTEA[]
    events JSON NOT NULL
)