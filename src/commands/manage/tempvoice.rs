use serenity::all::ChannelId;

use crate::{models::guild_settings::GuildSettings, Context, Error};

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
    let guild = cx.guild_id().unwrap();
    if let Some(channel) = channel {
        GuildSettings::set(
            &mut cx.data().database.get()?,
            guild,
            "creator_voice_channel",
            Some(channel.get().to_string()),
        )?;
        cx.say(format!(
            "The creator voice channel has been set to <#{}>",
            channel.get()
        ))
        .await?;
    } else {
        GuildSettings::set(&mut cx.data().database.get()?, guild, "creator_voice_channel", None::<String>)?;
        cx.say("The temporary voice channel creation has been disabled.")
            .await?;
    }
    Ok(())
}
