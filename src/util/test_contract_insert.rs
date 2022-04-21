use bigdecimal::BigDecimal;
pub use cw20::Cw20ExecuteMsg;
use dao_indexer::db::models::NewContract;
use std::str::FromStr;
use num_bigint::BigInt;
use diesel::RunQueryDsl;

fn test_contract_insert(db: &PgConnection) {
  use dao_indexer::db::schema::contracts::dsl::*;
  let big_u128 = u128::MAX - 10;
  dbg!(big_u128);
  let super_big_int = BigInt::from(big_u128) * BigInt::from(big_u128);
  let myheight = BigDecimal::from(super_big_int.clone());
  let supposed_height = BigInt::from_str(
      "115792089237316195423570985008687907845783772593379917843263342644414228988025",
  )
  .unwrap();
  dbg!(supposed_height == super_big_int);
  dbg!(BigInt::from(big_u128) * BigInt::from(big_u128));
  let contract = NewContract {
      address: "foo",
      staking_contract_address: "bar",
      code_id: -1,
      creator: "gavin",
      admin: "admin_foo",
      label: "label_foo",
      creation_time: "000",
      height: &myheight,
  };
  diesel::insert_into(contracts)
      .values(contract)
      .execute(db)
      .unwrap();
}
