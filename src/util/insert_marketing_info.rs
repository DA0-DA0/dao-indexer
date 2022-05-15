pub use cw20::Cw20ExecuteMsg;
use diesel::pg::PgConnection;
use diesel::prelude::*;

pub fn insert_marketing_info(
    db: &PgConnection,
    marketing_project: &str,
    marketing_description: &str,
    marketing_body_text: &str,
) -> QueryResult<i32> {
    use crate::db::schema::marketing::dsl::*;
    diesel::insert_into(marketing)
        .values((
            project.eq(marketing_project),
            description.eq(marketing_description),
            marketing_text.eq(marketing_body_text),
        ))
        .returning(id)
        .get_result(db)
}
