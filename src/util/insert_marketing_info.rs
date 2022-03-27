pub use cw20::Cw20ExecuteMsg;
use cw20_base::msg::InstantiateMarketingInfo;
use diesel::pg::PgConnection;
use diesel::prelude::*;

pub fn insert_marketing_info(
    db: &PgConnection,
    marketing_info: &InstantiateMarketingInfo,
) -> QueryResult<i32> {
    use crate::db::schema::marketing::dsl::*;
    diesel::insert_into(marketing)
        .values((
            project.eq(&marketing_info.project),
            description.eq(&marketing_info.description),
            marketing_text.eq(&marketing_info.marketing),
        ))
        .returning(id)
        .get_result(db)
}
