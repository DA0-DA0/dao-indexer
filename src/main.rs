use bigdecimal::BigDecimal;
use cosmrs::proto::cosmos::bank::v1beta1::MsgSend;
use cosmrs::proto::cosmos::base::v1beta1::Coin;
use cosmrs::proto::cosmwasm::wasm::v1::{MsgExecuteContract, MsgInstantiateContract};
use cosmrs::tx::{MsgProto, Tx};
use cosmwasm_std::Uint128;
use cw20::Cw20Coin;
pub use cw20::Cw20ExecuteMsg;
use cw20_base::msg::InstantiateMarketingInfo;
use cw3_dao::msg::{
    ExecuteMsg as Cw3DaoExecuteMsg, GovTokenMsg, InstantiateMsg as Cw3DaoInstantiateMsg,
};
use dao_indexer::db::connection::establish_connection;
use dao_indexer::db::models::{Cw20, Dao, NewContract};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use futures::StreamExt;
use serde_json::Value;
use stake_cw20::msg::ExecuteMsg as StakeCw20ExecuteMsg;
use std::collections::BTreeMap;
use std::str::FromStr;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient};

fn parse_message(msg: &[u8]) -> serde_json::Result<Option<Value>> {
    if let Ok(exec_msg_str) = String::from_utf8(msg.to_owned()) {
        if let Ok(parsed_json) = serde_json::from_str(&exec_msg_str) {
            return Ok(parsed_json);
        }
    }
    Ok(None)
}

fn index_message(
    _db: &PgConnection,
    sender: &str,
    contract_addr: &str,
    funds: &[Coin],
    msg: Option<&Vec<u8>>,
) {
    let mut json_dump: String = "".to_string();
    if let Some(msg) = msg {
        if let Ok(Some(parsed)) = parse_message(msg) {
            let obj = parsed.as_object();
            json_dump = serde_json::to_string_pretty(&obj).unwrap();
        }
    }
    println!(
        "{{\"sender\": \"{}\", \"contract_address\": \"{}\", \"funds\": \"{:?}\", \"contract\": {}}}",
        sender,
        contract_addr,
        funds,
        json_dump
    );
}

trait Index {
    fn index(&self, db: &PgConnection, events: &Option<BTreeMap<String, Vec<String>>>);
}

#[derive(Debug)]
struct ContractAddresses {
    dao_address: Option<String>,
    cw20_address: Option<String>,
    staking_contract_address: Option<String>,
}

fn get_contract_addresses(events: &Option<BTreeMap<String, Vec<String>>>) -> ContractAddresses {
    let mut dao_address = None;
    let mut cw20_address = None;
    let mut staking_contract_address = None;
    if let Some(transaction_events) = events {
        if let Some(addr) = transaction_events.get("instantiate._contract_address") {
            // 0: DAO
            // 1: cw20
            // 2: staking contract
            // But if you use an existing token, you'll just get
            // DAO/staking contract
            dao_address = Some(addr[0].clone());
            cw20_address = Some(addr[1].clone());
            staking_contract_address = Some(addr[2].clone());
        }
    }
    ContractAddresses {
        dao_address,
        cw20_address,
        staking_contract_address,
    }
}

fn insert_contract(db: &PgConnection, contract_model: &NewContract) {
    use dao_indexer::db::schema::contracts::dsl::*;
    diesel::insert_into(contracts)
        .values(contract_model)
        .execute(db)
        .expect("Error saving new post");
}

fn insert_marketing_info(
    db: &PgConnection,
    marketing_info: &InstantiateMarketingInfo,
) -> QueryResult<i32> {
    use dao_indexer::db::schema::marketing::dsl::*;
    diesel::insert_into(marketing)
        .values((
            project.eq(&marketing_info.project),
            description.eq(&marketing_info.description),
            marketing_text.eq(&marketing_info.marketing),
        ))
        .returning(id)
        .get_result(db)
}

fn update_balance(
    db: &PgConnection,
    tx_height: &BigDecimal,
    token_addr: &str,
    token_sender_address: &str,
    balance_update: &Cw20Coin,
) -> QueryResult<usize> {
    use dao_indexer::db::schema::cw20_transactions::dsl::*;
    let amount_converted: BigDecimal = BigDecimal::from(balance_update.amount.u128() as i64);
    diesel::insert_into(cw20_transactions)
        .values((
            cw20_address.eq(token_addr),
            sender_address.eq(token_sender_address),
            recipient_address.eq(&balance_update.address),
            height.eq(tx_height),
            amount.eq(amount_converted),
        ))
        .execute(db)
}

fn insert_gov_token(
    db: &PgConnection,
    token_msg: &GovTokenMsg,
    contract_addresses: &ContractAddresses,
    height: &BigDecimal,
) -> QueryResult<i32> {
    use dao_indexer::db::schema::gov_token::dsl::*;
    let result: QueryResult<i32>;
    match token_msg {
        GovTokenMsg::InstantiateNewCw20 {
            /*cw20_code_id, stake_contract_code_id, label,*/ msg,
            ..
        } => {
            let mut marketing_record_id: Option<i32> = None;
            if let Some(marketing) = &msg.marketing {
                marketing_record_id = Some(insert_marketing_info(db, marketing).unwrap());
            }
            let cw20_address = contract_addresses.cw20_address.as_ref().unwrap();
            result = diesel::insert_into(gov_token)
                .values((
                    name.eq(&msg.name),
                    address.eq(cw20_address),
                    symbol.eq(&msg.symbol),
                    decimals.eq(msg.decimals as i32),
                    marketing_id.eq(marketing_record_id),
                ))
                .returning(id)
                .get_result(db);
            let dao_address = contract_addresses.dao_address.as_ref().unwrap();
            if let Ok(_token_id) = result {
                for balance in &msg.initial_balances {
                    let _ = update_balance(db, height, cw20_address, dao_address, balance);
                }
            }
        }
        GovTokenMsg::UseExistingCw20 {
            addr,
            stake_contract_code_id,
            label,
            unstaking_duration,
        } => {
            println!("TODO: Use existing cw20 addr: {}, stake_contract_code_id: {}, label: {}, unstaking_duration: {:?}", addr, stake_contract_code_id, label, unstaking_duration);
            result = Ok(0);
        }
    };
    result
}

fn get_dao(db: &PgConnection, dao_address: &str) -> QueryResult<Dao> {
    use dao_indexer::db::schema::dao::dsl::*;
    dao.filter(contract_address.eq(dao_address))
        .first::<Dao>(db)
}

fn get_gov_token(db: &PgConnection, dao_address: &str) -> diesel::QueryResult<Cw20> {
    use dao_indexer::db::schema::gov_token::dsl::*;
    let dao = get_dao(db, dao_address).unwrap();
    gov_token.filter(id.eq(dao.gov_token_id)).first(db)
}

fn insert_dao(
    db: &PgConnection,
    instantiate_dao: &Cw3DaoInstantiateMsg,
    contract_addr: &ContractAddresses,
    height: &BigDecimal,
) {
    use dao_indexer::db::schema::dao::dsl::*;

    let inserted_token_id: i32 =
        insert_gov_token(db, &instantiate_dao.gov_token, contract_addr, height).unwrap();

    diesel::insert_into(dao)
        .values((
            name.eq(&instantiate_dao.name),
            contract_address.eq(contract_addr.dao_address.as_ref().unwrap()),
            staking_contract_address.eq(contract_addr.staking_contract_address.as_ref().unwrap()),
            description.eq(&instantiate_dao.description),
            gov_token_id.eq(inserted_token_id),
        ))
        .execute(db)
        .expect("Error saving dao");
}

impl Index for MsgInstantiateContract {
    fn index(&self, db: &PgConnection, events: &Option<BTreeMap<String, Vec<String>>>) {
        let contract_addresses = get_contract_addresses(events);
        println!(
            "cw20 {}, staking {}",
            contract_addresses.cw20_address.as_ref().unwrap(),
            contract_addresses
                .staking_contract_address
                .as_ref()
                .unwrap()
        );
        let dao_address = contract_addresses.dao_address.as_ref().unwrap();
        let staking_contract_address = contract_addresses.staking_contract_address.as_ref().unwrap();
        let mut tx_height = BigDecimal::from_str("0").unwrap();
        if let Some(event_map) = events {
            let tx_height_strings = event_map.get("tx.height").unwrap();
            let tx_height_str = &tx_height_strings[0];
            tx_height = BigDecimal::from_str(tx_height_str).unwrap();
        }

        let contract_model = NewContract::from_msg(dao_address, staking_contract_address, &tx_height, self);
        insert_contract(db, &contract_model);
        let msg_str = String::from_utf8(self.msg.clone()).unwrap();
        match serde_json::from_str::<Cw3DaoInstantiateMsg>(&msg_str) {
            Ok(instantiate_dao) => {
                insert_dao(db, &instantiate_dao, &contract_addresses, &tx_height);
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        };
    }
}

fn dump_execute_contract(execute_contract: &Cw3DaoExecuteMsg) {
    println!("handle execute contract {:?}", execute_contract);
}

fn dump_events(events: &Option<BTreeMap<String, Vec<String>>>) {
    if let Some(event_map) = events {
        println!("************* vv Events ***********");
        for (key, value) in event_map {
            println!("{} / {:?}", key, value);
        }
        println!("************* ^^ Events ***********");
    }
}

fn update_balance_from_events(
    db: &PgConnection,
    i: usize,
    event_map: &BTreeMap<String, Vec<String>>,
) -> QueryResult<usize> {
    let tx_height_string = &event_map.get("tx.height").unwrap()[0];
    let tx_height = BigDecimal::from_str(tx_height_string).unwrap();
    let amount = &event_map.get("wasm.amount").unwrap()[i];
    let receiver = &event_map.get("wasm.to").unwrap()[i];
    let sender = &event_map.get("wasm.sender").unwrap()[0];
    let from = &event_map.get("wasm.from").unwrap()[0]; // DAO address
    let gov_token = get_gov_token(db, from).unwrap();
    let balance_update = Cw20Coin {
        address: receiver.clone(),
        amount: Uint128::from_str(&amount).unwrap(),
    };
    update_balance(db, &tx_height, &gov_token.address, sender, &balance_update)
}

// fn update_balance_from_cw20_execute_events(
//     db: &PgConnection,
//     i: usize,
//     event_map: &BTreeMap<String, Vec<String>>,
// ) -> QueryResult<usize> {
//     let tx_height_string = &event_map.get("tx.height").unwrap()[0];
//     let tx_height = BigDecimal::from_str(tx_height_string).unwrap();
//     let amount = &event_map.get("wasm.amount").unwrap()[i];
//     let receiver = &event_map.get("wasm.to").unwrap()[0];
//     let sender = &event_map.get("wasm.from").unwrap()[i]; // user
//     let gov_token_address = &event_map.get("wasm._contract_address").unwrap()[0];
//     let balance_update = Cw20Coin {
//         address: receiver.clone(),
//         amount: Uint128::from_str(&amount).unwrap(),
//     };
//     update_balance(db, &tx_height, &gov_token_address, sender, &balance_update)
// }

impl Index for Cw3DaoExecuteMsg {
    fn index(&self, db: &PgConnection, events: &Option<BTreeMap<String, Vec<String>>>) {
        dump_execute_contract(&self);
        dump_events(events);
        if let Some(event_map) = events {
            if let Some(wasm_actions) = event_map.get("wasm.action") {
                // TODO(gavin.doughtie): Handle propose, vote
                if wasm_actions.len() > 0 && wasm_actions[0] == "execute" {
                    for (i, action_type) in (&wasm_actions[1..]).iter().enumerate() {
                        match action_type.as_str() {
                            "transfer" => {
                                update_balance_from_events(db, i, &event_map).unwrap();
                            }
                            "mint" => {
                                update_balance_from_events(db, i, &event_map).unwrap();
                            }
                            _ => {
                                eprintln!("Unhandled exec type {}", action_type);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Index for StakeCw20ExecuteMsg {
    fn index(&self, _db: &PgConnection, events: &Option<BTreeMap<String, Vec<String>>>) {
        println!("StakeCw20ExecuteMsg index");
        dump_events(events);
    }
}

impl Index for Cw20ExecuteMsg {
    fn index(&self, db: &PgConnection, events: &Option<BTreeMap<String, Vec<String>>>) {
        dump_events(events);
        if let Some(event_map) = events {
            if let Some(wasm_actions) = event_map.get("wasm.action") {
                if wasm_actions.len() > 0 && &wasm_actions[0] == "send" {
                    let tx_height =
                        BigDecimal::from_str(&(event_map.get("tx.height").unwrap()[0])).unwrap();
                    let contract_addresses = event_map.get("wasm._contract_address").unwrap();
                    let gov_token_address = &contract_addresses[0];
                    let to_addresses = event_map.get("wasm.to").unwrap();
                    let staking_contract_addr = to_addresses[0].clone();
                    let amounts = &event_map.get("wasm.amount").unwrap();
                    let senders = event_map.get("wasm.from").unwrap();
                    let sender_addr = &senders[0];
                    let mut send_amount: &str = &amounts[0];

                    let receiving_contract_action: &str;
                    if wasm_actions.len() > 1 {
                        receiving_contract_action = &wasm_actions[1];
                    } else {
                        receiving_contract_action = "";
                    }
                    let action_amount: &str = &amounts[1];
                    if receiving_contract_action == "stake" {
                        send_amount = action_amount;
                    }
                    let balance_update: Cw20Coin = Cw20Coin {
                        address: staking_contract_addr,
                        amount: Uint128::from_str(send_amount).unwrap(),
                    };
                    let _ = update_balance(
                        db,
                        &tx_height,
                        gov_token_address,
                        sender_addr,
                        &balance_update,
                    );
                }
            }
        }
    }
}

impl Index for MsgExecuteContract {
    fn index(&self, db: &PgConnection, events: &Option<BTreeMap<String, Vec<String>>>) {
        let msg_str = String::from_utf8(self.msg.clone()).unwrap();
        let mut errors = vec![];
        match serde_json::from_str::<Cw3DaoExecuteMsg>(&msg_str) {
            Ok(execute_contract) => {
                return execute_contract.index(db, events);
            }
            Err(e) => {
                errors.push(e);
            }
        };
        match serde_json::from_str::<StakeCw20ExecuteMsg>(&msg_str) {
            Ok(execute_contract) => {
                return execute_contract.index(db, events);
            }
            Err(e) => {
                errors.push(e);
            }
        };
        match serde_json::from_str::<Cw20ExecuteMsg>(&msg_str) {
            Ok(execute_contract) => {
                return execute_contract.index(db, events);
            }
            Err(e) => {
                errors.push(e);
            }
        }
        println!("could not interpret execute msg, got errors:\n{:?}", errors);
    }
}

impl Index for MsgSend {
    fn index(&self, db: &PgConnection, _events: &Option<BTreeMap<String, Vec<String>>>) {
        index_message(db, &self.from_address, &self.to_address, &self.amount, None);
    }
}

#[tokio::main]
async fn main() {
    let db: PgConnection = establish_connection();
    let (client, driver) = WebSocketClient::new("ws://127.0.0.1:26657/websocket")
        .await
        .unwrap();
    let driver_handle = tokio::spawn(async move { driver.run().await });

    // Subscribe to transactions (can also add blocks but just Tx for now)
    let mut subs = client.subscribe(EventType::Tx.into()).await.unwrap();

    while let Some(res) = subs.next().await {
        let ev = res.unwrap();
        let result = ev.data;
        let events = ev.events;
        match result {
            EventData::NewBlock { block, .. } => println!("{:?}", block.unwrap()),
            EventData::Tx { tx_result, .. } => match Tx::from_bytes(&tx_result.tx) {
                Ok(tx_parsed) => {
                    for msg in tx_parsed.body.messages {
                        let type_url: &str = &msg.type_url;
                        println!("instantiate msg: {}", type_url);
                        match type_url {
                            "/cosmwasm.wasm.v1.MsgInstantiateContract" => {
                                let msg_obj: MsgInstantiateContract =
                                    MsgProto::from_any(&msg).unwrap();
                                msg_obj.index(&db, &events);
                            }
                            "/cosmwasm.wasm.v1.MsgExecuteContract" => {
                                let msg_obj: MsgExecuteContract = MsgProto::from_any(&msg).unwrap();
                                msg_obj.index(&db, &events);
                            }
                            "/cosmos.bank.v1beta1.MsgSend" => {
                                let msg_obj: MsgSend = MsgProto::from_any(&msg).unwrap();
                                msg_obj.index(&db, &events);
                            }
                            _ => {
                                println!("No handler for {}", type_url);
                            }
                        }
                    }
                }
                Err(err) => println!("ERROR: {:?}", err),
            },
            _ => println!("unexpected result"),
        }
    }

    // Signal to the driver to terminate.
    match client.close() {
        Ok(val) => println!("closed {:?}", val),
        Err(e) => println!("Error closing client {:?}", e),
    }
    // Await the driver's termination to ensure proper connection closure.
    let _ = driver_handle.await.unwrap();
}
