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
        height -> Numeric,
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
    cw20_transactions (id) {
        id -> Int4,
        cw20_address -> Text,
        sender_address -> Text,
        recipient_address -> Text,
        amount -> Numeric,
        height -> Numeric,
    }
}

table! {
    dao (id) {
        id -> Int4,
        contract_address -> Text,
        name -> Text,
        description -> Text,
        image_url -> Nullable<Text>,
        gov_token_id -> Int4,
    }
}

table! {
    exec_msg (id) {
        id -> Int4,
        sender -> Text,
        address -> Text,
    }
}

table! {
    gov_token (id) {
        id -> Int4,
        address -> Text,
        name -> Text,
        symbol -> Text,
        decimals -> Nullable<Int4>,
        marketing_id -> Nullable<Int4>,
    }
}

table! {
    logo (id) {
        id -> Int4,
        url -> Nullable<Text>,
        svg -> Nullable<Text>,
        png -> Nullable<Bytea>,
    }
}

table! {
    marketing (id) {
        id -> Int4,
        project -> Nullable<Text>,
        description -> Nullable<Text>,
        marketing_text -> Nullable<Text>,
        logo_id -> Nullable<Int4>,
    }
}

allow_tables_to_appear_in_same_query!(
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
);
