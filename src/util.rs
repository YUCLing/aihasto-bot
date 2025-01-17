use std::collections::HashMap;

use diesel::{ExpressionMethods, RunQueryDsl};
use serenity::all::{CacheHttp, ChannelId, Context, CreateMessage, GuildId, Message};

use crate::{
    data::ConnectionPoolKey, models::moderation_log::ModerationLog, ConnectionPool, Error,
};

pub fn parse_duration_to_seconds<T: AsRef<str>>(duration: T) -> Result<u64, String> {
    let mut total_seconds = 0;
    let mut current_number = String::new();

    // Iterate through each character of the input string
    for ch in duration.as_ref().chars() {
        match ch {
            '0'..='9' => {
                // Collect digits for the current number
                current_number.push(ch);
            }
            's' | 'm' | 'h' | 'd' | 'w' => {
                // If we encounter a time unit (s, m, or h), process the current number
                if let Ok(value) = current_number.parse::<u64>() {
                    match ch {
                        's' => total_seconds += value,
                        'm' => total_seconds += value * 60,
                        'h' => total_seconds += value * 3600,
                        'd' => total_seconds += value * 86400,
                        'w' => total_seconds += value * 604800,
                        _ => return Err(format!("Unsupported unit: {}", ch)),
                    }
                    current_number.clear(); // Reset the current number for the next part
                } else {
                    return Err("Invalid number".to_string());
                }
            }
            _ => return Err("Invalid character in duration string".to_string()), // Handle invalid characters
        }
    }

    // After finishing the loop, if there's any leftover number, assume it's in seconds
    if !current_number.is_empty() {
        if let Ok(value) = current_number.parse::<u64>() {
            total_seconds += value; // Treat as seconds if no unit was given
        } else {
            return Err("Invalid number".to_string());
        }
    }

    Ok(total_seconds)
}

pub async fn get_pool_from_serenity(cx: &Context) -> ConnectionPool {
    cx.data
        .read()
        .await
        .get::<ConnectionPoolKey>()
        .unwrap()
        .clone()
}

pub async fn send_moderation_logs<
    T: CacheHttp,
    U: Into<ChannelId>,
    V: IntoIterator<Item = ModerationLog>,
>(
    cx: &T,
    channel: U,
    logs: V,
) -> Result<HashMap<ModerationLog, Message>, Error> {
    let channel: ChannelId = channel.into();
    let mut map = HashMap::new();
    for log in logs {
        let msg = channel
            .send_message(cx, CreateMessage::new().embed(log.clone().into()))
            .await?;
        map.insert(log, msg);
    }
    Ok(map)
}

pub async fn send_moderation_logs_with_database_records<
    T: CacheHttp,
    U: Into<GuildId>,
    V: Into<ChannelId>,
    W: IntoIterator<Item = ModerationLog>,
>(
    pool: &ConnectionPool,
    cx: &T,
    guild_id: U,
    channel_id: V,
    logs: W,
) -> Result<HashMap<ModerationLog, Message>, Error> {
    use crate::schema::moderation_log_message::*;
    use diesel::dsl::*;
    let guild_id: GuildId = guild_id.into();
    let channel_id: ChannelId = channel_id.into();
    let map = send_moderation_logs(cx, channel_id, logs).await?;
    insert_into(table)
        .values(
            map.iter()
                .map(|(log, msg)| {
                    (
                        id.eq(TryInto::<i64>::try_into(msg.id.get()).unwrap()),
                        log_id.eq(log.id),
                        guild.eq(TryInto::<i64>::try_into(guild_id.get()).unwrap()),
                        channel.eq(TryInto::<i64>::try_into(channel_id.get()).unwrap()),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .execute(&mut pool.get()?)?;
    Ok(map)
}
