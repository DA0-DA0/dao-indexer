CREATE TABLE codes (
    code_id BIGINT NOT NULL UNIQUE PRIMARY KEY,
    creator TEXT NOT NULL DEFAULT '',
    creation_time TEXT NOT NULL DEFAULT '',
    height BIGINT NOT NULL
);

CREATE INDEX codes_creator_index ON codes (creator);

CREATE TABLE contracts (
    address TEXT NOT NULL UNIQUE PRIMARY KEY,
    staking_contract_address TEXT NOT NULL,
    code_id BIGINT NOT NULL,
    creator TEXT NOT NULL DEFAULT '',
    admin TEXT NOT NULL DEFAULT '',
    label TEXT NOT NULL DEFAULT '',
    creation_time TEXT NOT NULL DEFAULT '',
    height NUMERIC(78) NOT NULL
);

CREATE INDEX contracts_code_id_index ON contracts (code_id);

CREATE INDEX contracts_creator_index ON contracts (creator);

CREATE TABLE exec_msg (
    id SERIAL PRIMARY KEY,
    sender TEXT NOT NULL,
    address TEXT NOT NULL
);

CREATE TABLE cw20_balances (
    id SERIAL PRIMARY KEY,
    address TEXT NOT NULL,
    token TEXT NOT NULL,
    balance NUMERIC(78) NOT NULL
);

CREATE TABLE cw20_transactions (
    id SERIAL PRIMARY KEY,
    cw20_address TEXT NOT NULL,
    sender_address TEXT NOT NULL,
    recipient_address TEXT NOT NULL,
    amount NUMERIC(78) NOT NULL,
    height NUMERIC(78) NOT NULL -- time?
);

CREATE TABLE coin (id SERIAL PRIMARY KEY);

CREATE TABLE dao (
    id SERIAL PRIMARY KEY,
    contract_address TEXT NOT NULL,
    staking_contract_address TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    image_url TEXT,
    gov_token_id INT NOT NULL
);

CREATE INDEX dao_staking_address_index on dao (staking_contract_address);

CREATE INDEX dao_contract_address_index on dao (contract_address);

CREATE TABLE marketing (
    id SERIAL PRIMARY KEY,
    project TEXT,
    description TEXT,
    marketing_text TEXT,
    logo_id INT
);

CREATE TABLE gov_token (
    id SERIAL PRIMARY KEY,
    address TEXT NOT NULL,
    name TEXT NOT NULL,
    symbol TEXT NOT NULL,
    decimals INT,
    marketing_id INT
);

CREATE INDEX gov_token_address_index on gov_token (address);

CREATE TABLE logo (
    id SERIAL PRIMARY KEY,
    url TEXT,
    svg TEXT,
    png BYTEA
);

-- pub struct InstantiateMarketingInfo {
--     pub project: Option<String>,
--     pub description: Option<String>,
--     pub marketing: Option<String>,
--     pub logo: Option<Logo>,
-- }
-- pub struct GovTokenInstantiateMsg {
--     pub name: String,
--     pub symbol: String,
--     pub decimals: u8,
--     pub initial_balances: Vec<Cw20Coin>,
--     pub marketing: Option<InstantiateMarketingInfo>,
-- }
-- InstantiateNewCw20 {
--     cw20_code_id: u64,
--     stake_contract_code_id: u64,
--     label: String,
--     initial_dao_balance: Option<Uint128>,
--     msg: GovTokenInstantiateMsg,
--     unstaking_duration: Option<Duration>,
-- },
-- pub struct InstantiateMsg {
--     // The name of the DAO.
--     pub name: String,
--     // A description of the DAO.
--     pub description: String,
--     /// Set an existing governance token or launch a new one
--     pub gov_token: GovTokenMsg,
--     /// Voting params configuration
--     pub threshold: Threshold,
--     /// The amount of time a proposal can be voted on before expiring
--     pub max_voting_period: Duration,
--     /// Deposit required to make a proposal
--     pub proposal_deposit_amount: Uint128,
--     /// Refund a proposal if it is rejected
--     pub refund_failed_proposals: Option<bool>,
--     /// Optional Image URL that is used by the contract
--     pub image_url: Option<String>,
-- }
CREATE
OR REPLACE FUNCTION update_balance_totals() RETURNS TRIGGER AS $balance_update$
BEGIN
    INSERT INTO
        cw20_balances(address, token, balance)
    VALUES
        (NEW.sender_address, NEW.cw20_address, NEW.amount);

    RETURN NEW;
END;

$balance_update$ LANGUAGE plpgsql;

CREATE
OR REPLACE TRIGGER transaction_trigger
AFTER
INSERT
    ON cw20_transactions FOR EACH ROW EXECUTE FUNCTION update_balance_totals();

-- INSERT INTO cw20_transactions(cw20_address, sender_address, recipient_address, amount, height) VALUES ('cw20_address', 'sender_address', 'recipient_address', 11, 12);
