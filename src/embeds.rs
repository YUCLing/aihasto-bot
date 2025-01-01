use serenity::all::{Colour, CreateEmbed, CreateEmbedFooter};

use crate::models::moderation_log::{ModerationAction, ModerationLog};

impl From<ModerationLog> for CreateEmbed {
    fn from(value: ModerationLog) -> Self {
        let mut embed = CreateEmbed::new()
            .description(value.reason.unwrap_or("No reason given.".to_string()))
            .footer(CreateEmbedFooter::new(format!(
                "ID: {}",
                value.id.to_string()
            )));
        if let Some(actor) = value.actor {
            embed = embed.field("Moderator", format!("<@{}>", actor), false);
        }
        embed = embed.fields([
            (
                "Created at",
                value
                    .created_at
                    .format("%Y-%m-%d %H:%M:%S (UTC)")
                    .to_string(),
                true,
            ),
            (
                "Updated at",
                value
                    .updated_at
                    .and_then(|x| Some(x.format("%Y-%m-%d %H:%M:%S (UTC)").to_string()))
                    .unwrap_or("Not yet updated".to_string()),
                true,
            ),
        ]);
        match value.kind {
            ModerationAction::Warning => {
                embed = embed.color(Colour::ORANGE).title("🔔 Warning");
            }
            ModerationAction::Flood => {
                embed = embed.color(Colour::LIGHT_GREY).title("🔒 Flood");
            }
            ModerationAction::Timeout => {
                embed = embed.color(Colour::PURPLE).title("🔇 Timeout");
            }
            ModerationAction::Ban => {
                embed = embed.color(Colour::RED).title("🚫 Ban");
            }
        };
        embed
    }
}
