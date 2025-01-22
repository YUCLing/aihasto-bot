use cache::{CacheKey, GuildSettingsCache};
use diesel::{
    insert_into,
    prelude::{Insertable, Queryable},
    query_dsl::methods::{FilterDsl, SelectDsl},
    ExpressionMethods, QueryResult, RunQueryDsl, Selectable, SelectableHelper,
};
use lazy_static::lazy_static;
use serenity::all::GuildId;

use crate::{schema::guild_settings, ConnectionPool};

mod cache;

lazy_static! {
    static ref GUILD_SETTINGS_CACHE: GuildSettingsCache = GuildSettingsCache::new(50);
}

#[derive(Insertable, Selectable, Queryable)]
#[diesel(table_name = crate::schema::guild_settings)]
pub struct GuildSettings {
    guild: i64,
    key: String,
    value: Option<String>,
}

impl GuildSettings {
    pub fn set<G: Into<GuildId>, K: AsRef<str>, V: AsRef<str>>(
        pool: &ConnectionPool,
        guild: G,
        key: K,
        value: Option<V>,
    ) -> QueryResult<usize> {
        let raw_id = guild.into().get();
        let key = key.as_ref().to_string();
        let cache_key = CacheKey {
            id: raw_id,
            name: key.clone(),
        };
        GUILD_SETTINGS_CACHE.invalidate(&cache_key);
        let id: i64 = raw_id.try_into().unwrap();
        let value = value.map(|x| x.as_ref().to_string());
        let result = insert_into(guild_settings::table)
            .values(GuildSettings {
                guild: id,
                key,
                value: value.clone(),
            })
            .on_conflict((guild_settings::guild, guild_settings::key))
            .do_update()
            .set(guild_settings::value.eq(value.clone()))
            .execute(&mut pool.get().map_err(|_| diesel::result::Error::NotFound)?);
        if let (Some(value), Ok(_)) = (value, &result) {
            GUILD_SETTINGS_CACHE.insert(cache_key, value);
        }
        result
    }

    pub fn get<G: Into<GuildId>, K: AsRef<str>>(
        pool: &ConnectionPool,
        guild: G,
        key: K,
    ) -> Option<String> {
        let raw_id = guild.into().get();
        let key = key.as_ref().to_string();
        if let Some(value) = GUILD_SETTINGS_CACHE.get(&CacheKey {
            id: raw_id,
            name: key.clone(),
        }) {
            return Some(value);
        }
        let id: i64 = raw_id.try_into().unwrap();
        let Ok(mut conn) = pool.get() else {
            return None;
        };
        let Ok(result): Result<GuildSettings, _> = guild_settings::table
            .filter(guild_settings::guild.eq(id))
            .filter(guild_settings::key.eq(key))
            .select(GuildSettings::as_select())
            .get_result(&mut conn)
        else {
            return None;
        };
        result.value
    }
}
