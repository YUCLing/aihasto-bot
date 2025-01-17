use diesel::{delete, ExpressionMethods, RunQueryDsl};
use serenity::{
    all::{
        ActivityData, AuditLogEntry, ChannelId, ChannelType, Context, EventHandler, GuildChannel,
        GuildId, Interaction, Message, MessageId, MessageUpdateEvent, Ready, VoiceState,
    },
    async_trait,
};

use crate::{
    features::{message_change_log, moderation, moderation_log, temp_voice},
    schema::voice_channels,
    util::get_pool_from_serenity,
};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, cx: Context, ready: Ready) {
        log::info!(
            "Bot {} is connected!",
            ready
                .user
                .global_name
                .clone()
                .unwrap_or(ready.user.name.clone())
        );
        cx.set_presence(
            Some(ActivityData::playing("Catridges")),
            serenity::all::OnlineStatus::DoNotDisturb,
        );
    }

    async fn channel_delete(
        &self,
        cx: Context,
        channel: GuildChannel,
        _messages: Option<Vec<Message>>,
    ) {
        if channel.kind == ChannelType::Voice {
            // try delete voice channel record.
            if let Ok(mut conn) = get_pool_from_serenity(&cx).await.get() {
                delete(voice_channels::table)
                    .filter(
                        voice_channels::id.eq(TryInto::<i64>::try_into(channel.id.get()).unwrap()),
                    )
                    .execute(&mut conn)
                    .expect("Unable to delete voice channel record.");
            }
        }
    }

    async fn guild_audit_log_entry_create(
        &self,
        cx: Context,
        entry: AuditLogEntry,
        guild_id: GuildId,
    ) {
        tokio::spawn(moderation_log::guild_audit_log_entry_create(
            cx, entry, guild_id,
        ));
    }

    async fn message_delete(
        &self,
        cx: Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        guild_id: Option<GuildId>,
    ) {
        tokio::spawn(message_change_log::handle_message_delete(
            cx,
            channel_id,
            deleted_message_id,
            guild_id,
        ));
    }

    async fn message_delete_bulk(
        &self,
        cx: Context,
        channel_id: ChannelId,
        multiple_deleted_messages_ids: Vec<MessageId>,
        guild_id: Option<GuildId>,
    ) {
        tokio::spawn(message_change_log::handle_message_delete_bulk(
            cx,
            channel_id,
            multiple_deleted_messages_ids,
            guild_id,
        ));
    }

    async fn message_update(
        &self,
        cx: Context,
        old_if_available: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        tokio::spawn(message_change_log::handle_message_update(
            cx,
            old_if_available,
            new,
            event,
        ));
    }

    async fn interaction_create(&self, cx: Context, interaction: Interaction) {
        tokio::spawn(temp_voice::handle_interaction(
            cx.clone(),
            interaction.clone(),
        ));
        tokio::spawn(moderation::handle_interaction(cx, interaction));
    }

    async fn voice_state_update(&self, cx: Context, _old: Option<VoiceState>, new: VoiceState) {
        tokio::spawn(temp_voice::handle_voice_state_update(cx, new));
    }
}
