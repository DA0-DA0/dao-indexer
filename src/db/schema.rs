table! {
    codes (code_id) {
        code_id -> Int8,
        creator -> Text,
        creation_time -> Text,
        height -> Int8,
    }
}

table! {
    coin (id) {
        id -> Int4,
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
    dao (id) {
        id -> Int4,
        name -> Text,
        description -> Text,
        image_url -> Nullable<Text>,
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
    codes,
    coin,
    contracts,
    cw20_balances,
    dao,
    exec_msg,
);
