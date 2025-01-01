use diesel::{
    backend::Backend,
    deserialize::FromSql,
    expression::AsExpression,
    insert_into,
    prelude::{Insertable, Queryable},
    query_builder::IncompleteInsertStatement,
    serialize::ToSql,
    sql_types::Text,
    Selectable,
};
use serenity::all::{GuildId, UserId};
use uuid::Uuid;

use crate::schema::{moderation_log, sql_types::ModerationAction as SqlModerationAction};

#[derive(Debug, AsExpression)]
#[diesel(sql_type = SqlModerationAction)]
pub enum ModerationAction {
    Warning,
    Flood,
    Timeout,
    Ban,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::moderation_log)]
pub struct CreateModerationLog {
    guild: i64,
    kind: ModerationAction,
    member: i64,
    actor: Option<i64>,
    reason: Option<String>,
}

impl CreateModerationLog {
    pub fn new<G: Into<GuildId>, U: Into<UserId>, A: Into<UserId>, R: AsRef<str>>(
        guild: G,
        kind: ModerationAction,
        member: U,
        actor: Option<A>,
        reason: Option<R>,
    ) -> Self {
        CreateModerationLog {
            guild: guild.into().get().try_into().unwrap(),
            kind,
            member: member.into().get().try_into().unwrap(),
            actor: actor.and_then(|x| Some(x.into().get().try_into().unwrap())),
            reason: reason.and_then(|x| Some(x.as_ref().to_string())),
        }
    }
}

#[allow(dead_code)]
#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::moderation_log)]
pub struct ModerationLog {
    id: Uuid,
    guild: i64,
    kind: ModerationAction,
    member: i64,
    actor: Option<i64>,
    reason: Option<String>,
}

impl ModerationLog {
    pub fn insert() -> IncompleteInsertStatement<moderation_log::table> {
        insert_into(moderation_log::table)
    }
}

impl<DB> ToSql<SqlModerationAction, DB> for ModerationAction
where
    DB: Backend,
    str: ToSql<Text, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        match self {
            ModerationAction::Warning => "warning",
            ModerationAction::Flood => "flood",
            ModerationAction::Timeout => "timeout",
            ModerationAction::Ban => "ban",
        }
        .to_sql(out)
    }
}

impl<DB> FromSql<SqlModerationAction, DB> for ModerationAction
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: <DB as Backend>::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        match String::from_sql(bytes)?.as_str() {
            "warning" => Ok(ModerationAction::Warning),
            "flood" => Ok(ModerationAction::Flood),
            "timeout" => Ok(ModerationAction::Timeout),
            "ban" => Ok(ModerationAction::Ban),
            x => Err(format!("Unknown variant {}", x).into()),
        }
    }
}
