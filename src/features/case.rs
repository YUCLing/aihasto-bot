use std::str::FromStr;

use diesel::{delete as diesel_delete, ExpressionMethods, OptionalExtension, RunQueryDsl};
use serenity::all::{CacheHttp, ChannelId, MessageId};
use uuid::Uuid;

use crate::{acquire_cache_http, ConnectionPool, Error};

pub async fn delete_impl<T>(pool: &ConnectionPool, case_id: T) -> Result<String, Error>
where
    T: AsRef<str>,
{
    use crate::schema::moderation_log::*;
    let uuid = Uuid::from_str(case_id.as_ref()).map_err(|_| "Case ID is invalid.")?;
    let result: Option<(i64, i64)> = {
        use crate::schema::moderation_log_message::*;
        diesel_delete(table)
            .filter(log_id.eq(uuid))
            .returning((id, channel))
            .get_result(&mut pool.get()?)
            .optional()?
    };
    if let Some((message_id, channel_id)) = result {
        let channel = ChannelId::new(channel_id.try_into().unwrap());
        channel
            .delete_message(
                acquire_cache_http().http(),
                MessageId::new(message_id.try_into().unwrap()),
            )
            .await?;
    }
    let count = diesel_delete(table)
        .filter(id.eq(uuid))
        .execute(&mut pool.get()?)?;
    if count == 0 {
        return Ok("No case with provided ID found.".to_string());
    };
    Ok("Case has been deleted.".to_string())
}
