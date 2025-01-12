use serenity::all::{CreateEmbed, CreateEmbedFooter};

use crate::models::moderation_log::ModerationLog;

impl From<ModerationLog> for CreateEmbed {
    fn from(value: ModerationLog) -> Self {
        let mut embed = value
            .kind
            .create_embed()
            .description(value.reason.unwrap_or("No reason given.".to_string()))
            .field("User", format!("<@{}>", value.member), true)
            .footer(CreateEmbedFooter::new(format!("ID: {}", value.id)));
        if let Some(actor) = value.actor {
            embed = embed.field("Moderator", format!("<@{}>", actor), true);
        }
        embed = embed.fields([
            ("\t", "\t".to_string(), true),
            (
                "Created at",
                format!("<t:{}>", value.created_at.and_utc().timestamp()),
                true,
            ),
            (
                "Updated at",
                value
                    .updated_at
                    .map(|x| format!("<t:{}>", x.and_utc().timestamp()))
                    .unwrap_or("Not yet updated".to_string()),
                true,
            ),
        ]);
        embed
    }
}
