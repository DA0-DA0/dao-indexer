CREATE TABLE codes (
    code_id BIGINT NOT NULL UNIQUE PRIMARY KEY,
    creator TEXT NOT NULL DEFAULT '',
    creation_time TEXT NOT NULL DEFAULT '',
    height BIGINT NOT NULL
);

CREATE INDEX codes_creator_index ON codes (creator);

CREATE TABLE contracts (
    address TEXT NOT NULL UNIQUE PRIMARY KEY,
    code_id BIGINT NOT NULL,
    creator TEXT NOT NULL DEFAULT '',
    admin TEXT NOT NULL DEFAULT '',
    label TEXT NOT NULL DEFAULT '',
    creation_time TEXT NOT NULL DEFAULT '',
    height BIGINT NOT NULL,
    json JSONB NOT NULL DEFAULT '{}'
);

CREATE INDEX contracts_code_id_index ON contracts (code_id);

CREATE INDEX contracts_creator_index ON contracts (creator);

CREATE TABLE exec_msg (
    id SERIAL PRIMARY KEY,
    sender TEXT NOT NULL,
    address TEXT NOT NULL,
    funds JSONB,
    json JSONB
);

CREATE TABLE cw20_balances (
    id SERIAL PRIMARY KEY,
    address TEXT NOT NULL,
    token TEXT NOT NULL,
    balance BIGINT NOT NULL
);
