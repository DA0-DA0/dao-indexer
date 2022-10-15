// @generated automatically by Diesel CLI.

diesel::table! {
    block (height) {
        height -> Int8,
        hash -> Text,
        num_txs -> Nullable<Int8>,
    }
}

diesel::table! {
    codes (code_id) {
        code_id -> Int8,
        creator -> Text,
        creation_time -> Text,
        height -> Int8,
    }
}

diesel::table! {
    coin (id) {
        id -> Int4,
    }
}

diesel::table! {
    contracts (address) {
        address -> Text,
        staking_contract_address -> Text,
        code_id -> Int8,
        creator -> Text,
        admin -> Text,
        label -> Text,
        creation_time -> Text,
        height -> Numeric,
    }
}

diesel::table! {
    cw20_balances (id) {
        id -> Int4,
        address -> Text,
        token -> Text,
        balance -> Numeric,
    }
}

diesel::table! {
    cw20_transactions (id) {
        id -> Int4,
        cw20_address -> Text,
        sender_address -> Text,
        recipient_address -> Text,
        amount -> Numeric,
        height -> Numeric,
    }
}

diesel::table! {
    dao (contract_address) {
        contract_address -> Text,
        staking_contract_address -> Text,
        name -> Text,
        description -> Text,
        image_url -> Nullable<Text>,
        gov_token_address -> Nullable<Text>,
        is_multisig -> Nullable<Bool>,
    }
}

diesel::table! {
    exec_msg (id) {
        id -> Int4,
        sender -> Text,
        address -> Text,
    }
}

diesel::table! {
    gov_token (address) {
        address -> Text,
        name -> Text,
        symbol -> Text,
        decimals -> Nullable<Int4>,
        marketing_id -> Nullable<Int4>,
    }
}

diesel::table! {
    logo (id) {
        id -> Int4,
        url -> Nullable<Text>,
        svg -> Nullable<Text>,
        png -> Nullable<Bytea>,
    }
}

diesel::table! {
    marketing (id) {
        id -> Int4,
        project -> Nullable<Text>,
        description -> Nullable<Text>,
        marketing_text -> Nullable<Text>,
        logo_id -> Nullable<Int4>,
    }
}

diesel::table! {
    transaction (hash) {
        hash -> Text,
        height -> Int8,
        response -> Jsonb,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    block,
    codes,
    coin,
    contracts,
    cw20_balances,
    cw20_transactions,
    dao,
    exec_msg,
    gov_token,
    logo,
    marketing,
    transaction,
);
