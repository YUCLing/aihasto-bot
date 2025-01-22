use serenity::all::{
    Attachment, ChannelId, Colour, Context, CreateEmbed, CreateEmbedFooter, CreateMessage, GuildId,
    Message, MessageId, MessageUpdateEvent,
};

use crate::{models::guild_settings::GuildSettings, util::get_pool_from_serenity};

pub async fn handle_message_delete(
    cx: Context,
    channel_id: ChannelId,
    deleted_message_id: MessageId,
    guild_id: Option<GuildId>,
) {
    let Some(guild_id) = guild_id else {
        return;
    };
    let Some(cached_msg) = cx
        .cache
        .message(channel_id, deleted_message_id)
        .map(|x| x.clone())
    else {
        // we missed out the message...
        return;
    };
    if let Some(log_channel) = GuildSettings::get(
        &get_pool_from_serenity(&cx).await,
        guild_id,
        "message_change_log_channel",
    )
    .map(|x| ChannelId::new(x.parse().unwrap()))
    {
        let mut footer = vec![format!("ID: {}", cached_msg.id)];
        let mut embed = CreateEmbed::new()
            .color(Colour::RED)
            .title("Message Deleted")
            .fields([
                ("User", format!("<@{}>", cached_msg.author.id), true),
                ("Channel", format!("<#{}>", cached_msg.channel_id), true),
                (
                    "Sent at",
                    format!("<t:{}>", cached_msg.timestamp.timestamp()),
                    false,
                ),
            ])
            .author(cached_msg.author.into())
            .description(cached_msg.content);
        if !cached_msg.attachments.is_empty() {
            embed = embed.field(
                "Attachments",
                cached_msg
                    .attachments
                    .iter()
                    .map(|x| x.url.clone())
                    .collect::<Vec<String>>()
                    .join("\n"),
                false,
            );
            footer.push("Attachments may already deleted by Discord.".to_string());
        }
        embed = embed.footer(CreateEmbedFooter::new(footer.join(" • ")));
        tokio::spawn(log_channel.send_message(cx.clone(), CreateMessage::new().embed(embed)));
    }
}

pub async fn handle_message_delete_bulk(
    cx: Context,
    channel_id: ChannelId,
    multiple_deleted_messages_ids: Vec<MessageId>,
    guild_id: Option<GuildId>,
) {
    for msg in multiple_deleted_messages_ids {
        tokio::spawn(handle_message_delete(cx.clone(), channel_id, msg, guild_id))
            .await
            .unwrap();
    }
}

fn find_attachments_diff(old: Vec<Attachment>, new: Vec<Attachment>) -> Vec<Attachment> {
    let mut diff_elements = Vec::new();

    for item1 in &old {
        if new.iter().all(|item2| item1.id != item2.id) {
            diff_elements.push(item1.clone());
        }
    }

    diff_elements
}

pub async fn handle_message_update(
    cx: Context,
    old_if_available: Option<Message>,
    new: Option<Message>,
    event: MessageUpdateEvent,
) {
    let Some(old_message) = old_if_available else {
        // still, we missed the message.
        return;
    };
    let Some(msg) = new else {
        // where is the message?
        return;
    };
    let Some(edited_timestamp) = msg.edited_timestamp else {
        // editing embed doesn't add an edited timestamp (probably)
        return;
    };
    let Some(guild_id) = event.guild_id else {
        return;
    };
    if let Some(log_channel) = GuildSettings::get(
        &get_pool_from_serenity(&cx).await,
        guild_id,
        "message_change_log_channel",
    )
    .map(|x| ChannelId::new(x.parse().unwrap()))
    {
        let author = event
            .author
            .expect("Why an edited message doesn't have an author.");
        let removed_attachments = find_attachments_diff(old_message.attachments, msg.attachments);
        let mut footer = vec![format!("ID: {}", msg.id)];
        let mut embed = CreateEmbed::new()
            .color(Colour::ORANGE)
            .title("Message Edited")
            .fields([
                ("User", format!("<@{}>", author.id), true),
                ("Channel", format!("<#{}>", msg.channel_id), true),
                (
                    "Edited at",
                    format!("<t:{}>", edited_timestamp.timestamp()),
                    false,
                ),
            ])
            .author(author.into())
            .description(if old_message.content != msg.content {
                format!(
                    "**_Old Message_**\n\
                    {}\n\n\
                    **_New Message_**\n\
                    {}",
                    old_message.content, msg.content
                )
            } else {
                "_No content changes._".to_string()
            });
        if !removed_attachments.is_empty() {
            embed = embed.field(
                "Removed Attachments",
                removed_attachments
                    .iter()
                    .map(|x| x.url.clone())
                    .collect::<Vec<String>>()
                    .join("\n"),
                false,
            );
            footer.push("Attachments may already deleted by Discord.".to_string());
        }
        embed = embed.footer(CreateEmbedFooter::new(footer.join(" • ")));
        log_channel
            .send_message(cx.clone(), CreateMessage::new().embed(embed))
            .await
            .expect("Cannot send message update log.");
    }
}
