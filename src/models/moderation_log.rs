use chrono::NaiveDateTime;
use diesel::{
    backend::Backend,
    deserialize::{FromSql, FromSqlRow},
    dsl::{AsSelect, SqlTypeOf},
    expression::AsExpression,
    insert_into,
    pg::Pg,
    prelude::{Insertable, Queryable},
    query_builder::IncompleteInsertStatement,
    serialize::ToSql,
    sql_types::Text,
    ExpressionMethods, QueryDsl, Selectable, SelectableHelper,
};
use serenity::all::{Colour, CreateEmbed, GuildId, UserId};
use uuid::Uuid;

use crate::schema::{moderation_log, sql_types::ModerationAction as SqlModerationAction};

#[derive(Debug, Clone, AsExpression, FromSqlRow, PartialEq, Eq, Hash)]
#[diesel(sql_type = SqlModerationAction)]
pub enum ModerationAction {
    Warning,
    Flood,
    Timeout,
    Ban,
}

impl ModerationAction {
    pub fn embed_title(&self) -> &str {
        match self {
            Self::Warning => "🔔 Warning",
            Self::Flood => "🔒 Flood",
            Self::Timeout => "🔇 Timeout",
            Self::Ban => "🚫 Ban",
        }
    }

    pub fn embed_color(&self) -> Colour {
        match self {
            Self::Warning => Colour::ORANGE,
            Self::Flood => Colour::LIGHT_GREY,
            Self::Timeout => Colour::PURPLE,
            Self::Ban => Colour::RED,
        }
    }

    pub fn create_embed(&self) -> CreateEmbed {
        CreateEmbed::new()
            .color(self.embed_color())
            .title(self.embed_title())
    }
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
            actor: actor.map(|x| x.into().get().try_into().unwrap()),
            reason: reason.map(|x| x.as_ref().to_string()),
        }
    }
}

#[allow(dead_code)]
#[derive(Queryable, Selectable, Clone, PartialEq, Eq, Hash)]
#[diesel(table_name = crate::schema::moderation_log)]
pub struct ModerationLog {
    pub id: Uuid,
    pub guild: i64,
    pub kind: ModerationAction,
    pub member: i64,
    pub actor: Option<i64>,
    pub reason: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

impl ModerationLog {
    pub fn insert() -> IncompleteInsertStatement<moderation_log::table> {
        insert_into(moderation_log::table)
    }

    pub fn all() -> moderation_log::BoxedQuery<'static, Pg, SqlTypeOf<AsSelect<ModerationLog, Pg>>>
    {
        moderation_log::table
            .select(ModerationLog::as_select())
            .into_boxed()
    }

    #[diesel::dsl::auto_type(no_type_alias)]
    pub fn by_kind(kind: ModerationAction) -> _ {
        moderation_log::kind.eq(kind)
    }

    #[diesel::dsl::auto_type(no_type_alias)]
    pub fn by_user<U: Into<UserId>>(user: U) -> _ {
        let id: i64 = TryInto::<i64>::try_into(user.into().get()).unwrap();
        moderation_log::member.eq(id)
    }

    #[diesel::dsl::auto_type(no_type_alias)]
    pub fn by_actor<U: Into<UserId>>(user: U) -> _ {
        let id: i64 = TryInto::<i64>::try_into(user.into().get()).unwrap();
        moderation_log::actor.eq(id)
    }

    #[diesel::dsl::auto_type(no_type_alias)]
    pub fn no_actor() -> _ {
        moderation_log::actor.is_null()
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
