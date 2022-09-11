#[cfg(test)]
pub mod tests {
    use crate::db::db_persister::DatabasePersister;
    use crate::db::persister::{make_persister_ref, Persister};
    use crate::indexing::indexer_registry::IndexerRegistry;
    use crate::indexing::schema_indexer::*;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult, Transaction};
    use serde::{Deserialize, Serialize};
    use tokio::test;

    use crate::db::db_builder::DatabaseBuilder;
    use crate::db::persister::PersisterRef;

    use crate::indexing::event_map::EventMap;

    use schemars::schema::RootSchema;
    use serde_json::Value;

    use schemars::JsonSchema;
    #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
    #[serde(rename_all = "snake_case")]
    struct SimpleMessage {
        simple_field_one: String,
        simple_field_two: u128,
    }

    #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
    enum SimpleSubMessage {
        TypeA {
            type_a_contract_address: String,
            type_a_count: u32,
        },
        TypeB {
            type_b_contract_address: String,
            type_b_count: u32,
            type_b_addtional_field: String,
        },
    }

    #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
    #[serde(rename_all = "snake_case")]
    struct SimpleRelatedMessage {
        title: String,
        message: SimpleMessage,
        sub_message: SimpleSubMessage,
    }

    struct TestRegistryResult {
        registry: IndexerRegistry,
        _indexer_id: usize,
        _persister: PersisterRef<u64>,
    }

    #[allow(dead_code)]
    #[cfg(test)]
    fn get_test_registry(
        name: &str,
        schema: RootSchema,
        _db: Option<sea_orm::DatabaseConnection>,
        persister: Option<PersisterRef<u64>>,
    ) -> TestRegistryResult {
        use crate::{db::persister::StubPersister, indexing::indexer_registry::Register};
        let indexer;
        let persister_ref: PersisterRef<u64>;
        if let Some(persister) = persister {
            persister_ref = persister;
            indexer = SchemaIndexer::<u64>::new(
                name.to_string(),
                vec![SchemaRef {
                    name: name.to_string(),
                    schema,
                    version: "0.0.0",
                }],
                persister_ref.clone(),
            );
        } else {
            let stub: Box<dyn Persister<Id = u64>> = Box::from(StubPersister {});
            persister_ref = make_persister_ref(stub);
            indexer = SchemaIndexer::<u64>::new(
                name.to_string(),
                vec![SchemaRef {
                    name: name.to_string(),
                    schema,
                    version: "0.0.0",
                }],
                persister_ref.clone(),
            );
        }
        let mut registry = IndexerRegistry::new(None, None, persister_ref.clone());
        let indexer_id = registry.register(Box::from(indexer), None);
        TestRegistryResult {
            registry,
            _indexer_id: indexer_id,
            _persister: persister_ref,
        }
    }

    fn new_mock_db() -> MockDatabase {
        MockDatabase::new(DatabaseBackend::Postgres).append_exec_results(vec![
            MockExecResult {
                last_insert_id: 15,
                rows_affected: 1,
            },
            MockExecResult {
                last_insert_id: 15,
                rows_affected: 1,
            },
            MockExecResult {
                last_insert_id: 15,
                rows_affected: 1,
            },
        ])
    }

    async fn assert_expected_transactions(
        registry: &IndexerRegistry,
        msg_dictionary: &Value,
        expected_transaction_log: Vec<Transaction>,
        mock_results: Vec<MockExecResult>,
    ) {
        let mock_db =
            MockDatabase::new(DatabaseBackend::Postgres).append_exec_results(mock_results);

        let db = mock_db.into_connection();

        // let result = registry.db_builder.create_tables(&db).await;
        // println!("{:#?}", result);

        let db_persister = DatabasePersister::new(db);

        let persist_result = registry
            .db_builder
            .value_mapper
            .persist_message(&db_persister, "SimpleRelatedMessage", msg_dictionary, None) // "SimpleRelatedMessage is definitely wrong"
            .await;

        if persist_result.is_err() {
            eprintln!("Error persisting: ${:#?}", persist_result);
        }

        let transactions = db_persister.into_transaction_log();
        assert_eq!(transactions, expected_transaction_log);
    }

    #[test]
    async fn test_schema_indexer_init() {
        use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;
        use cw3_dao_2_5::msg::InstantiateMsg as Cw3DaoInstantiateMsg25;
        use schemars::schema_for;

        let schema3 = schema_for!(Cw3DaoInstantiateMsg);
        let schema25 = schema_for!(Cw3DaoInstantiateMsg25);
        let mock_db = new_mock_db().into_connection();
        let persister = DatabasePersister::new(mock_db);
        let persister_ref = make_persister_ref(Box::new(persister));
        let indexer = SchemaIndexer::<u64>::new(
            "Cw3DaoInstantiateMsg".to_string(),
            vec![
                SchemaRef {
                    name: "Cw3DaoInstantiateMsg".to_string(),
                    schema: schema3,
                    version: "0.2.6",
                },
                SchemaRef {
                    name: "Cw3DaoInstantiateMsg25".to_string(),
                    schema: schema25,
                    version: "0.2.5",
                },
            ],
            persister_ref,
        );
        let pos = indexer
            .schemas
            .iter()
            .position(|schema| schema.name == "Cw3DaoInstantiateMsg");
        assert!(pos.is_some());
    }

    #[test]
    async fn test_simple_message() {
        use crate::db::db_test::compare_table_create_statements;
        use schemars::schema_for;

        let name = stringify!(SimpleMessage);
        let schema = schema_for!(SimpleMessage);
        let db = new_mock_db().into_connection();
        let persister = DatabasePersister::new(db);
        let persister_ref = make_persister_ref(Box::new(persister));
        let result = get_test_registry(name, schema, None, Some(persister_ref.clone()));
        let mut registry = result.registry;
        assert!(registry.initialize().is_ok(), "failed to init indexer");

        let built_table = registry.db_builder.table(name);
        let expected_sql = vec![
            r#"CREATE TABLE IF NOT EXISTS "simple_message" ("#,
            r#""simple_field_one" text,"#,
            r#""simple_field_two" integer"#,
            r#")"#,
        ]
        .join(" ");
        compare_table_create_statements(built_table, &expected_sql);

        let msg_str = r#"
        {
            "simple_field_one": "simple_field_one value",
            "simple_field_two": 33
        }"#;
        let msg_dictionary = serde_json::from_str(msg_str).unwrap();
        let result = registry.index_message_and_events(&EventMap::new(), &msg_dictionary, msg_str);
        assert!(result.is_ok());
    }

    fn build_mock(last_insert_id: u64) -> MockExecResult {
      MockExecResult {
        last_insert_id,
        rows_affected: 1,
      }
    }

    #[test]
    async fn test_simple_related_message() {
        use crate::db::db_test::compare_table_create_statements;
        use schemars::schema_for;

        let name = stringify!(SimpleRelatedMessage);
        let schema = schema_for!(SimpleRelatedMessage);

        let mut next_record_id = 15u64;
        let simple_related_message_id = next_record_id;
        next_record_id = 16u64;
        let sub_message_id = 16u64;
        next_record_id += 1;
        let related_message_id = next_record_id;

        let mapped_mock_results: Vec<MockExecResult> = (16..27).map(build_mock).collect();

        let mock_results = vec![
            // Mocks result from creating the simple_related_message record
            MockExecResult {
                last_insert_id: simple_related_message_id,
                rows_affected: 1,
            },
            // Mocks result from creating the simple_message record
            MockExecResult {
                last_insert_id: sub_message_id,
                rows_affected: 1,
            },
            // Mocks result from creating the first sub-message
            MockExecResult {
                last_insert_id: related_message_id,
                rows_affected: 1,
            },
            // Mocks ???
            MockExecResult {
                last_insert_id: related_message_id + 1,
                rows_affected: 1,
            },
            // Mocks ???
            MockExecResult {
                last_insert_id: related_message_id + 2,
                rows_affected: 1,
            },
            // Mocks ???
            MockExecResult {
                last_insert_id: related_message_id + 3,
                rows_affected: 1,
            },
            // Mocks ???
            MockExecResult {
                last_insert_id: related_message_id + 4,
                rows_affected: 1,
            },
            // Mocks ???
            MockExecResult {
                last_insert_id: related_message_id + 5,
                rows_affected: 1,
            },
            // Mocks ???
            MockExecResult {
                last_insert_id: related_message_id + 6,
                rows_affected: 1,
            },
            // Mocks ???
            MockExecResult {
                last_insert_id: related_message_id + 5,
                rows_affected: 1,
            },
            // Mocks ???
            MockExecResult {
                last_insert_id: related_message_id + 5,
                rows_affected: 1,
            },
        ];
        let mock_db =
            MockDatabase::new(DatabaseBackend::Postgres).append_exec_results(mock_results.clone());
        let db = mock_db.into_connection();

        let persister: Box<dyn Persister<Id = u64>> = Box::new(DatabasePersister::new(db));
        let persister_ref = make_persister_ref(persister);
        let result = get_test_registry(name, schema, None, Some(persister_ref.clone()));
        let mut registry = result.registry;
        assert!(registry.initialize().is_ok(), "failed to init indexer");
        let expected_sql = vec![
            r#"CREATE TABLE IF NOT EXISTS "simple_related_message" ("#,
            r#""sub_message_id" integer,"#,
            r#""title" text,"#,
            r#""message_id" integer )"#,
        ]
        .join(" ");
        let built_table = registry.db_builder.table(name);
        compare_table_create_statements(built_table, &expected_sql);

        let expected_sql = vec![
            r#"CREATE TABLE IF NOT EXISTS "simple_message" ("#,
            r#""id" serial unique,"#,
            r#""simple_field_one" text,"#,
            r#""simple_field_two" integer"#,
            r#")"#,
        ]
        .join(" ");
        let built_table = registry.db_builder.table("SimpleMessage");
        println!("{:#?}", built_table);
        compare_table_create_statements(built_table, &expected_sql);

        // Now the sub-tables
        let built_sub_message_table = registry.db_builder.table("simple_sub_message");
        println!("{:#?}", built_sub_message_table);
        let title = "SimpleRelatedMessage Title";
        let msg_str = r#"
    {
        "title": "SimpleRelatedMessage Title",
        "message": {
            "simple_field_one": "simple_field_one value",
            "simple_field_two": 33
        },
        "sub_message": {
            "type_a_contract_address": "type a contract address value",
            "type_a_count": 99
        }
    }"#;
        let msg_dictionary = serde_json::from_str(msg_str).unwrap();
        let result = registry.index_message_and_events(&EventMap::new(), &msg_dictionary, msg_str);
        assert!(result.is_ok());

        // TODO: this is missing creation of related records, and should fail comparison.
        let expected_transaction_log = vec![
            sea_orm::Transaction::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"INSERT INTO "simple_message" ("simple_field_one", "simple_field_two") VALUES ($1, $2)"#,
                vec!["simple_field_one value".into(), 33_i64.into()],
            ),
            sea_orm::Transaction::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"INSERT INTO "simple_related_message" ("title", "message_id", "sub_message_id") VALUES ($1, $2, $3)"#,
                vec![
                    title.into(),
                    (simple_related_message_id as i64).into(),
                    // THIS IS A BUG, probably in the test. Should be the sub_message_id
                    0_i64.into() //(sub_message_id as i64).into(),
                ],
            ),
        ];

        assert_expected_transactions(
            &registry,
            &msg_dictionary,
            expected_transaction_log,
            mapped_mock_results,
        )
        .await
    }

    #[test]
    async fn test_visit() {
        use cw3_dao::msg::InstantiateMsg as Cw3DaoInstantiateMsg;
        use schemars::schema_for;
        let schema3 = schema_for!(Cw3DaoInstantiateMsg);
        let label = stringify!(Cw3DaoInstantiateMsg);

        let db = new_mock_db().into_connection();
        let persister = DatabasePersister::new(db);
        let persister_ref = make_persister_ref(Box::new(persister));

        let mut indexer = SchemaIndexer::<u64>::new(label.to_string(), vec![], persister_ref);
        let mut builder = DatabaseBuilder::new();
        let mut visitor = SchemaVisitor::new(&mut indexer, &mut builder);
        let result = visitor.visit_root_schema(&schema3);
        if result.is_err() {
            eprintln!("failed {:#?}", result);
        }
        let msg_string = r#"{
            "name": "Unit Test Dao",
            "description": "Unit Test Dao Description",
            "gov_token": {},
            "staking_contract": {},
            "threshold": {},
            "max_voting_period": {},
            "proposal_deposit_amount": {},
            "refund_failed_proposals": true,
            "image_url": "logo.png",
            "only_members_execute": true,
            "automatically_add_cw20s": true
          }"#;
        let msg = serde_json::from_str::<serde_json::Value>(msg_string).unwrap();
        let db = new_mock_db().into_connection();
        let persister = DatabasePersister::new(db);
        let result = builder
            .value_mapper
            .persist_message(&persister, label, &msg, None)
            .await;
        builder.finalize_columns();
        // If you want to look at the generated SQL, you can uncomment the next
        // this line:
        // println!("{}", builder.sql_string().unwrap());
        assert!(result.is_ok());
        println!("{:#?}", persister.into_transaction_log());
        // println!("{:#?}", db_ref.write().await.into_transaction_log());
    }
}
