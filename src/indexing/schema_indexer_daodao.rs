use crate::{build_and_register_schema_indexer, build_schema_ref};
use cw3_dao::msg::ExecuteMsg as Cw3DaoExecuteMsg;
use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;
use schemars::schema_for;
use stake_cw20::msg::ExecuteMsg as StakeCw20ExecuteMsg;

use crate::db::persister::PersisterRef;
use crate::indexing::indexer_registry::{IndexerRegistry, Register};
use crate::indexing::schema_indexer::SchemaIndexer;
use crate::indexing::schema_indexer::SchemaRef;
use cw20::Cw20ExecuteMsg;
use cw3_dao_2_5::msg::InstantiateMsg as Cw3DaoInstantiateMsg25;
use cw3_multisig::msg::ExecuteMsg as Cw3MultisigExecuteMsg;
use cw3_multisig::msg::InstantiateMsg as Cw3MultisigInstantiateMsg;

pub fn register_daodao_schema_indexers(
    registry: &mut IndexerRegistry,
    persister_ref: PersisterRef<u64>,
) -> anyhow::Result<()> {
    let cw3dao_indexer = SchemaIndexer::<u64>::new(
        "Cw3DaoInstantiateMsg".to_string(),
        vec![
            build_schema_ref!(Cw3DaoInstantiateMsg, "0.2.6"),
            build_schema_ref!(Cw3DaoInstantiateMsg25, "0.2.5"),
        ],
        persister_ref.clone(),
    );
    registry.register(Box::from(cw3dao_indexer), None);

    build_and_register_schema_indexer!(Cw3DaoExecuteMsg, "0.2.6", persister_ref, registry);
    build_and_register_schema_indexer!(Cw20ExecuteMsg, "0.13.4", persister_ref, registry);
    build_and_register_schema_indexer!(Cw3MultisigExecuteMsg, "0.2.5", persister_ref, registry);
    build_and_register_schema_indexer!(Cw3MultisigInstantiateMsg, "0.2.5", persister_ref, registry);
    build_and_register_schema_indexer!(StakeCw20ExecuteMsg, "0.2.4", persister_ref, registry);

    Ok(())
}
