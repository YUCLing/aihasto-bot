use diesel::{dsl::insert_into, RunQueryDsl};
use serenity::all::{CacheHttp, ChannelId, CreateChannel, GuildChannel, GuildId, Member, UserId};

use crate::{models::voice_channel::CreateVoiceChannel, schema::voice_channels, Connection, Error};

pub fn default_channel_name_for_member(member: &Member) -> String {
    member.nick.clone().unwrap_or(member.user.display_name().to_string())
}

pub async fn create_temp_voice_channel<
    U: CacheHttp,
    V: Into<GuildId>,
    W: Into<UserId>,
    X: AsRef<str>
>(conn: &mut Connection, http: &U, guild: V, creator: W, name: X, category: Option<ChannelId>) -> Result<GuildChannel, Error> {
    let mut create_channel = CreateChannel::new(name.as_ref().to_string())
        .kind(serenity::all::ChannelType::Voice);
    if let Some(category) = category {
        create_channel = create_channel.category(category);
    }
    let guild = guild.into();
    let channel = guild
        .create_channel(
            http,
            create_channel
        )
        .await;
    match channel {
        Ok(channel) => {
            insert_into(voice_channels::table)
                .values(&[
                    CreateVoiceChannel::new(&channel, guild, creator)
                ])
                .execute(conn)?;
            Ok(channel)
        }
        Err(err) => Err(Box::new(err))
    }
}