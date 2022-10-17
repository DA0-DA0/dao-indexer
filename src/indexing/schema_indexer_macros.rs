#[macro_export]
macro_rules! build_schema_ref {
    ($schema_class: ident, $schema_version: literal) => {
        SchemaRef {
            name: stringify!($schema_class).to_string(),
            schema: schema_for!($schema_class),
            version: stringify!($schema_version),
        }
    };
}

#[macro_export]
macro_rules! build_and_register_schema_indexer {
    ($schema_class: ident, $schema_version: literal, $persister_ref: ident, $registry: ident) => {
        let indexer = SchemaIndexer::<u64>::new(
            stringify!($schema_class).to_string(),
            vec![build_schema_ref!($schema_class, $schema_version)],
            $persister_ref.clone(),
        );
        $registry.register(Box::from(indexer), None);
    };
}
