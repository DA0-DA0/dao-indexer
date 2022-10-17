#[macro_export]
macro_rules! build_schema_ref {
  ($schema_class: ident, $schema_version: literal) => {
      // The macro will expand into the contents of this block.
      SchemaRef {
          name: stringify!($schema_class).to_string(),
          schema: schema_for!($schema_class),
          version: stringify!($schema_version),
      }
  };
}
