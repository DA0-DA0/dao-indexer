table! {
    block (height) {
        height -> Int8,
        hash -> Text,
        num_txs -> Nullable<Int8>,
    }
}

table! {
    codes (code_id) {
        code_id -> Int8,
        creator -> Text,
        creation_time -> Text,
        height -> Int8,
    }
}

table! {
    contracts (address) {
        address -> Text,
        code_id -> Int8,
        creator -> Text,
        admin -> Text,
        label -> Text,
        creation_time -> Text,
        height -> Int8,
        json -> Jsonb,
    }
}

table! {
    cw20_balances (id) {
        id -> Int4,
        address -> Text,
        token -> Text,
        balance -> Int8,
    }
}

table! {
    exec_msg (id) {
        id -> Int4,
        sender -> Text,
        address -> Text,
        funds -> Nullable<Jsonb>,
        json -> Nullable<Jsonb>,
    }
}

allow_tables_to_appear_in_same_query!(
    block,
    codes,
    contracts,
    cw20_balances,
    exec_msg,
);
