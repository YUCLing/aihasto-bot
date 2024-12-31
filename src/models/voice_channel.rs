use chrono::NaiveDateTime;
use diesel::{prelude::{Insertable, Queryable}, Selectable};
use serenity::all::{ChannelId, GuildId, UserId};

#[derive(Insertable)]
#[diesel(table_name = crate::schema::voice_channels)]
pub struct CreateVoiceChannel {
    id: i64,
    guild: i64,
    creator: i64
}

impl CreateVoiceChannel {
    pub fn new<T: Into<ChannelId>, G: Into<GuildId>, U: Into<UserId>>(channel_id: T, guild: G, creator: U) -> Self {
        CreateVoiceChannel {
            id: channel_id.into().get().try_into().unwrap(),
            guild: guild.into().get().try_into().unwrap(),
            creator: creator.into().get().try_into().unwrap(),
        }
    }
}

#[allow(dead_code)]
#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::voice_channels)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct VoiceChannel {
    pub id: i64,
    pub guild: i64,
    pub creator: i64,
    pub created_at: NaiveDateTime
}