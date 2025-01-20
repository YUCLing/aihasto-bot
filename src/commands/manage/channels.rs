use serenity::all::ChannelId;

use crate::{commands::manage::set_server_id_impl, Context, Error};

#[poise::command(
    slash_command,
    subcommands("set_moderation_log_channel", "set_message_change_log_channel")
)]
pub async fn channels(_cx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Set the moderation log channel.
#[poise::command(slash_command, guild_only, ephemeral)]
pub async fn set_moderation_log_channel(
    cx: Context<'_>,
    #[description = "The channel that will be the moderation log channel, ignore to disable"]
    #[channel_types("Text")]
    channel: Option<ChannelId>,
) -> Result<(), Error> {
    cx.say(
        set_server_id_impl(
            "moderation_log_channel",
            "moderation log channel",
            "#",
            &cx.data().database,
            cx.guild_id().unwrap(),
            channel,
        )
        .await?,
    )
    .await?;
    Ok(())
}

/// Set message change log channel.
#[poise::command(slash_command, guild_only, ephemeral)]
pub async fn set_message_change_log_channel(
    cx: Context<'_>,
    #[description = "The channel that will be the message change log channel, ignore to disable"]
    #[channel_types("Text")]
    channel: Option<ChannelId>,
) -> Result<(), Error> {
    cx.say(
        set_server_id_impl(
            "message_change_log_channel",
            "message change log channel",
            "#",
            &cx.data().database,
            cx.guild_id().unwrap(),
            channel,
        )
        .await?,
    )
    .await?;
    Ok(())
}
