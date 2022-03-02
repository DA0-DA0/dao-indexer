
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;


#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ContractInfo {}
}