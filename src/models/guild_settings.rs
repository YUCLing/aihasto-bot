use diesel::{
    insert_into,
    prelude::{Insertable, Queryable},
    query_dsl::methods::{FilterDsl, SelectDsl},
    ExpressionMethods, QueryResult, RunQueryDsl, Selectable, SelectableHelper,
};
use serenity::all::GuildId;

use crate::{schema::guild_settings, Connection};

#[derive(Insertable, Selectable, Queryable)]
#[diesel(table_name = crate::schema::guild_settings)]
pub struct GuildSettings {
    guild: i64,
    key: String,
    value: Option<String>,
}

impl GuildSettings {
    pub fn set<G: Into<GuildId>, K: AsRef<str>, V: AsRef<str>>(
        conn: &mut Connection,
        guild: G,
        key: K,
        value: Option<V>,
    ) -> QueryResult<usize> {
        let id: i64 = guild.into().get().try_into().unwrap();
        let key = key.as_ref().to_string();
        let value = value.and_then(|x| Some(x.as_ref().to_string()));
        insert_into(guild_settings::table)
            .values(GuildSettings {
                guild: id,
                key: key.clone(),
                value: value.clone(),
            })
            .on_conflict((guild_settings::guild, guild_settings::key))
            .do_update()
            .set(guild_settings::value.eq(value))
            .execute(conn)
    }

    pub fn get<G: Into<GuildId>, K: AsRef<str>>(
        conn: &mut Connection,
        guild: G,
        key: K,
    ) -> Option<String> {
        let id: i64 = guild.into().get().try_into().unwrap();
        let key = key.as_ref().to_string();
        let Ok(result): Result<GuildSettings, _> = guild_settings::table
            .filter(guild_settings::guild.eq(id))
            .filter(guild_settings::key.eq(key))
            .select(GuildSettings::as_select())
            .get_result(conn)
        else {
            return None;
        };
        result.value
    }
}
