use serenity::all::ChannelId;

use crate::{models::guild_settings::GuildSettings, Context, Error};

#[poise::command(slash_command, subcommands("set_log_channel"))]
pub async fn channels(_cx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Set the moderation log channel.
#[poise::command(slash_command, guild_only, ephemeral)]
pub async fn set_log_channel(
    cx: Context<'_>,
    #[description = "The channel that will be the moderation log channel, ignore to disable"]
    #[channel_types("Text")]
    channel: Option<ChannelId>,
) -> Result<(), Error> {
    let guild = cx.guild_id().unwrap();
    let mut conn = cx.data().database.get()?;
    if let Some(channel) = channel {
        GuildSettings::set(
            &mut conn,
            guild,
            "moderation_log_channel",
            Some(channel.get().to_string()),
        )?;
        cx.say(format!(
            "The moderation log channel has been set to <#{}>",
            channel.get()
        ))
        .await?;
    } else {
        GuildSettings::set(&mut conn, guild, "moderation_log_channel", None::<String>)?;
        cx.say("The moderation log channel creation has been disabled.")
            .await?;
    }
    Ok(())
}