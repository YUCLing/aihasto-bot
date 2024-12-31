use diesel::{delete, ExpressionMethods, RunQueryDsl};
use serenity::{
    all::{
        ActivityData, ChannelType, Context, EventHandler, GuildChannel, Interaction, Message,
        Ready, VoiceState,
    },
    async_trait,
};

use crate::{features::temp_voice, schema::voice_channels, ConnectionPoolKey};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, cx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        cx.set_presence(
            Some(ActivityData::playing("MiSide")),
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

    async fn voice_state_update(&self, cx: Context, _old: Option<VoiceState>, new: VoiceState) {
        tokio::spawn(temp_voice::handle_voice_state_update(cx, new));
    }

    async fn interaction_create(&self, cx: Context, interaction: Interaction) {
        tokio::spawn(temp_voice::handle_interaction(cx, interaction));
    }
}
