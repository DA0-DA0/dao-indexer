use diesel::pg::PgConnection;
use cosmrs::proto::cosmos::base::v1beta1::Coin;
use super::parse::parse_message;

pub fn index_message(
  _db: &PgConnection,
  sender: &str,
  contract_addr: &str,
  funds: &[Coin],
  msg: Option<&Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
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
  Ok(())
}
