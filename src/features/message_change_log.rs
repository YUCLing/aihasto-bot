use serenity::all::{
    Attachment, ChannelId, Colour, Context, CreateEmbed, CreateEmbedFooter, CreateMessage, GuildId,
    Message, MessageId, MessageUpdateEvent,
};

use crate::{data::ConnectionPoolKey, models::guild_settings::GuildSettings};

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
        .and_then(|x| Some(x.clone()))
    else {
        // we missed out the message...
        return;
    };
    let mut conn = cx
        .data
        .read()
        .await
        .get::<ConnectionPoolKey>()
        .unwrap()
        .get()
        .expect("Unable to get a database connection.");
    if let Some(log_channel) = GuildSettings::get(&mut conn, guild_id, "message_change_log_channel")
        .and_then(|x| Some(ChannelId::new(x.parse().unwrap())))
    {
        let mut embed = CreateEmbed::new()
            .color(Colour::RED)
            .title("Message Deleted")
            .fields([
                ("User", format!("<@{}>", cached_msg.author.id), true),
                (
                    "Sent at",
                    format!("<t:{}>", cached_msg.timestamp.timestamp()),
                    true,
                ),
            ])
            .author(cached_msg.author.into())
            .description(cached_msg.content);
        if !cached_msg.attachments.is_empty() {
            embed = embed
                .field(
                    "Attachments",
                    cached_msg
                        .attachments
                        .iter()
                        .map(|x| x.url.clone())
                        .collect::<Vec<String>>()
                        .join("\n"),
                    false,
                )
                .footer(CreateEmbedFooter::new(
                    "Attachments may already deleted by Discord.",
                ));
        }
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
        handle_message_delete(cx.clone(), channel_id, msg, guild_id).await;
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
    let Some(guild_id) = event.guild_id else {
        return;
    };
    let mut conn = cx
        .data
        .read()
        .await
        .get::<ConnectionPoolKey>()
        .unwrap()
        .get()
        .expect("Unable to get a database connection.");
    if let Some(log_channel) = GuildSettings::get(&mut conn, guild_id, "message_change_log_channel")
        .and_then(|x| Some(ChannelId::new(x.parse().unwrap())))
    {
        let author = event
            .author
            .expect("Why an edited message doesn't have an author.");
        let removed_attachments = find_attachments_diff(old_message.attachments, msg.attachments);
        let mut embed = CreateEmbed::new()
            .color(Colour::ORANGE)
            .title("Message Edited")
            .fields([
                ("User", format!("<@{}>", author.id), true),
                (
                    "Edited at",
                    format!(
                        "<t:{}>",
                        msg.edited_timestamp
                            .expect("Why an edited message doesn't contain a timestamp.") // todo: this does fail, need investigation...
                            .timestamp()
                    ),
                    true,
                ),
            ])
            .author(author.into())
            .description(format!(
                "**_Old Message_**\n\
                {}\n\n\
                **_New Message_**\n\
                {}",
                old_message.content, msg.content
            ));
        if !removed_attachments.is_empty() {
            embed = embed
                .field(
                    "Removed Attachments",
                    removed_attachments
                        .iter()
                        .map(|x| x.url.clone())
                        .collect::<Vec<String>>()
                        .join("\n"),
                    false,
                )
                .footer(CreateEmbedFooter::new(
                    "Attachments may already deleted by Discord.",
                ));
        }
        log_channel
            .send_message(cx.clone(), CreateMessage::new().embed(embed))
            .await
            .expect("Cannot send message update log.");
    }
}
