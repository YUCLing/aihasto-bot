use serenity::all::{ChannelId, CreateEmbedFooter, CreateMessage, User};

use crate::models::moderation_log::{ModerationAction, ModerationLog};

pub fn generate_dm_message<T: Into<ChannelId>>(
    log: &ModerationLog,
    moderator: &User,
    channel: Option<T>,
) -> CreateMessage {
    let mut embed = log
        .kind
        .create_embed()
        .author(moderator.clone().into())
        .description(log.reason.clone().unwrap_or("No reason given.".to_string()))
        .field("Moderator", format!("<@{}>", moderator.id.get()), true)
        .footer(CreateEmbedFooter::new(format!("ID: {}", log.id)));
    if let Some(channel) = channel {
        embed = embed.field("Channel", format!("<#{}>", channel.into().get()), true);
    }
    CreateMessage::new()
        .content(format!(
            "You are {} by a moderator from AIHASTO.",
            match log.kind {
                ModerationAction::Warning => "warned",
                ModerationAction::Flood => "marked as Flooder",
                ModerationAction::Timeout => "timedout",
                ModerationAction::Ban => "banned",
            }
        ))
        .embed(embed)
}
