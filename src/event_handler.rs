use diesel::{delete, ExpressionMethods, RunQueryDsl};
use serenity::{
    all::{
        ActivityData, AuditLogEntry, ChannelType, Context, EventHandler, GuildChannel, GuildId,
        Interaction, Message, Ready, VoiceState,
    },
    async_trait,
};

use crate::{
    data::{BotIdKey, ConnectionPoolKey},
    features::{moderation_log, temp_voice},
    schema::voice_channels,
};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, cx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        cx.data
            .write()
            .await
            .insert::<BotIdKey>(ready.user.id.get());
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
            let lck = cx.data.read().await;
            let pool = lck.get::<ConnectionPoolKey>().unwrap();
            if let Ok(mut conn) = pool.get() {
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

    async fn interaction_create(&self, cx: Context, interaction: Interaction) {
        tokio::spawn(temp_voice::handle_interaction(cx, interaction));
    }

    async fn voice_state_update(&self, cx: Context, _old: Option<VoiceState>, new: VoiceState) {
        tokio::spawn(temp_voice::handle_voice_state_update(cx, new));
    }
}
