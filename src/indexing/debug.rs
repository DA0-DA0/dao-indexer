use super::parse::parse_message;
use cosmrs::proto::cosmos::base::v1beta1::Coin;
use log::debug;

pub fn index_message(
    sender: &str,
    contract_addr: &str,
    funds: &[Coin],
    msg: Option<&Vec<u8>>,
) -> anyhow::Result<()> {
    let mut json_dump: String = "".to_string();
    if let Some(msg) = msg {
        if let Ok(Some(_parsed)) = parse_message(msg) {
            // let obj = parsed.as_object();
            // json_dump = serde_json::to_string_pretty(&obj).unwrap();
            json_dump = "[json_dump skipped]".to_string();
        }
    }
    debug!(
      "{{\"sender\": \"{}\", \"contract_address\": \"{}\", \"funds\": \"{:?}\", \"contract\": {}}}",
      sender,
      contract_addr,
      funds,
      json_dump
  );
    Ok(())
}
