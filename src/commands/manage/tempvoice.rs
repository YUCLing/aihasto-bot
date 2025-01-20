use serenity::all::ChannelId;

use crate::{commands::manage::set_server_id_impl, Context, Error};

#[poise::command(slash_command, guild_only, subcommands("set_channel"))]
pub async fn tempvoice(_cx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Set the creator channel for temporary voice.
#[poise::command(slash_command, guild_only, ephemeral)]
pub async fn set_channel(
    cx: Context<'_>,
    #[description = "The channel that will be the creator channel, ignore this to disable."]
    #[channel_types("Voice")]
    channel: Option<ChannelId>,
) -> Result<(), Error> {
    cx.say(
        set_server_id_impl(
            "creator_voice_channel",
            "creator voice channel",
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
