use std::str::FromStr;

use diesel::{
    delete as diesel_delete, update, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl,
    SelectableHelper,
};
use serenity::all::{ChannelId, CreateMessage, EditMessage, MessageId};
use uuid::Uuid;

use crate::{
    models::{guild_settings::GuildSettings, moderation_log::ModerationLog},
    Context, Error,
};

#[poise::command(
    slash_command,
    guild_only,
    subcommands("reason", "delete"),
    default_member_permissions = "MUTE_MEMBERS"
)]
pub async fn case(_cx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Update the reason of a case.
#[poise::command(slash_command, ephemeral, default_member_permissions = "MUTE_MEMBERS")]
pub async fn reason(
    cx: Context<'_>,
    #[description = "ID of the case to be updated"]
    #[rename = "id"]
    case_id: String,
    #[description = "New reason"]
    #[rename = "reason"]
    new_reason: String,
) -> Result<(), Error> {
    use crate::schema::moderation_log::*;
    let pool = &cx.data().database;
    let Some(log) = update(table)
        .filter(id.eq(Uuid::from_str(&case_id).map_err(|_| "Case ID is invalid.")?))
        .set((reason.eq(new_reason), updated_at.eq(diesel::dsl::now)))
        .returning(ModerationLog::as_returning())
        .get_result(&mut pool.get()?)
        .optional()?
    else {
        cx.say("No case with provided ID found.").await?;
        return Ok(());
    };
    if let Some(channel) =
        GuildSettings::get(pool, cx.guild_id().unwrap(), "moderation_log_channel")
    {
        let result: Option<(i64, i64, i64)> = {
            use crate::schema::moderation_log_message::*;
            table
                .filter(log_id.eq(log.id))
                .select((id, guild, channel))
                .get_result(&mut pool.get()?)
                .optional()?
        };
        let channel = ChannelId::new(channel.parse().unwrap());
        channel
            .send_message(
                &cx,
                if let Some((message_id, guild_id, channel_id)) = result {
                    channel.edit_message(&cx, MessageId::new(message_id.try_into().unwrap()), EditMessage::new()
                        .embed(log.into()))
                        .await?;
                    CreateMessage::new().content(format!(
                        "A case has been updated.\nLink to the case: https://discord.com/channels/{}/{}/{}",
                        guild_id, channel_id, message_id
                    ))
                } else {
                    CreateMessage::new()
                        .content("A case has been updated.")
                        .embed(log.into())
                },
            )
            .await?;
    }
    cx.say("Case has been updated.").await?;
    Ok(())
}

/// Delete a case.
#[poise::command(slash_command, ephemeral, default_member_permissions = "MUTE_MEMBERS")]
pub async fn delete(
    cx: Context<'_>,
    #[description = "ID of the case to be deleted"]
    #[rename = "id"]
    case_id: String,
) -> Result<(), Error> {
    use crate::schema::moderation_log::*;
    let pool = &cx.data().database;
    let Some(log) = diesel_delete(table)
        .filter(id.eq(Uuid::from_str(&case_id).map_err(|_| "Case ID is invalid.")?))
        .returning(ModerationLog::as_returning())
        .get_result(&mut pool.get()?)
        .optional()?
    else {
        cx.say("No case with provided ID found.").await?;
        return Ok(());
    };
    if let Some(channel) =
        GuildSettings::get(pool, cx.guild_id().unwrap(), "moderation_log_channel")
    {
        let result: Option<i64> = {
            use crate::schema::moderation_log_message::*;
            diesel_delete(table)
                .filter(log_id.eq(log.id))
                .returning(id)
                .get_result(&mut pool.get()?)
                .optional()?
        };
        let channel = ChannelId::new(channel.parse().unwrap());
        if let Some(message_id) = result {
            channel
                .delete_message(&cx, MessageId::new(message_id.try_into().unwrap()))
                .await?;
        }
    }
    cx.say("Case has been deleted.").await?;
    Ok(())
}
