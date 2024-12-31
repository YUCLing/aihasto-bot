use serenity::all::EditChannel;

use crate::{util::parse_duration_to_seconds, Context, Error};

/// Set or remove rate limit for a channel
/// 
/// Example Usage:
/// `/slowmode 2h` - Sets rate limit to 2 hours per message.
/// `/slowmode` - Removes rate limit
#[poise::command(
    slash_command,
    ephemeral,
    guild_only,
    category = "Moderation",
    required_bot_permissions = "MANAGE_CHANNELS",
    default_member_permissions = "MANAGE_CHANNELS"
)]
pub async fn slowmode(cx: Context<'_>,
    #[description = "Message cooldown in seconds (max 6 hours), leave empty or set to 0 to remove"]
    cooldown: Option<String>,
    #[description = "Channel to operate on, defaults to current channel"]
    #[channel_types("Text")]
    channel: Option<serenity::all::GuildChannel>
) -> Result<(), Error> {
    let cooldown = match
        parse_duration_to_seconds(
            cooldown.unwrap_or("0".to_string())
        ).and_then(|x|
            x.try_into().map_err(|_| "Invalid number".to_string())
        ) {
        Ok(x) => x,
        Err(err) => {
            cx.say(err).await?;
            return Ok(())
        }
    };
    cx.say(if cooldown == 0 { "Removing rate limit" } else { "Setting channel to slowmode" }).await?;
    let mut channel = channel.unwrap_or(cx.guild_channel().await.unwrap());
    channel.edit(cx, 
        EditChannel::new()
            .rate_limit_per_user(cooldown)
    ).await?;
    Ok(())
}

