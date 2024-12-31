use diesel::{
    delete,
    query_dsl::methods::{FilterDsl, LimitDsl, SelectDsl},
    ExpressionMethods, RunQueryDsl, SelectableHelper,
};
use serenity::{
    all::{
        ActivityData, ChannelId, ComponentInteractionDataKind, Context,
        CreateInteractionResponse, CreateInteractionResponseFollowup,
        CreateInteractionResponseMessage, CreateMessage, EventHandler, GuildChannel, Interaction,
        Message, Ready, VoiceState,
    },
    async_trait,
};

use crate::{
    features::temp_voice::{create_temp_voice_channel, default_channel_name_for_member}, models::voice_channel::VoiceChannel, schema::voice_channels, ConnectionPoolKey
};

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
        let lck = cx.data.read().await;
        let pool = lck.get::<ConnectionPoolKey>().unwrap();
        if let Ok(mut conn) = pool.get() {
            delete(voice_channels::table)
                .filter(voice_channels::id.eq(TryInto::<i64>::try_into(channel.id.get()).unwrap()))
                .execute(&mut conn)
                .expect("Unable to delete voice channel record.");
        }
    }

    async fn voice_state_update(&self, cx: Context, _old: Option<VoiceState>, new: VoiceState) {
        let lck = cx.data.read().await;
        let pool = lck.get::<ConnectionPoolKey>().unwrap();
        let mut conn = pool.get().unwrap();
            if let Some(channel_id) = new.channel_id {
                let guild_id = new.guild_id.unwrap();
                if channel_id.get() == 1321337141820653632u64 {
                    if let Some(member) = new.member {
                        let created_channels = voice_channels::table
                            .filter(
                                voice_channels::creator
                                    .eq(TryInto::<i64>::try_into(member.user.id.get()).unwrap()),
                            )
                            .limit(1)
                            .select(VoiceChannel::as_select())
                            .load(&mut conn)
                            .unwrap();
                        let mut co_conn = pool.get().unwrap(); // get a connection for coroutine
                        let create_channel_and_move_user = async {
                            let parent_channel = channel_id
                                .to_channel(&cx)
                                .await
                                .unwrap()
                                .guild()
                                .unwrap()
                                .parent_id;
                            match create_temp_voice_channel(&mut co_conn, &cx, &guild_id, member.user.id, default_channel_name_for_member(&member), parent_channel).await {
                                Ok(channel) => {
                                    match member.move_to_voice_channel(&cx, &channel).await {
                                        Ok(_) => {
                                            channel.send_message(&cx, CreateMessage::new()
                                                .content(format!(
                                                    "<@{}> You have just created a voice channel!\n\
                                                    \n\
                                                    You are the owner of this channel.\n\
                                                    The channel will only get deleted once there's no one here.\n\
                                                    If you left the channel while someone is still here, you can come back anytime.\n\
                                                    \n\
                                                    Note that you are unable to create another new channel unless this channel is deleted.",
                                                    member.user.id.get()
                                                ))
                                            ).await.expect("Unable to send message");
                                        }
                                        Err(_) => {
                                            // user could probably quit the voice channel while creating, just delete the new created channel.
                                            channel.delete(&cx).await.unwrap();
                                        }
                                    };
                                },
                                Err(err) => {
                                    eprintln!("Failed to create voice channel: {err:?}");
                                },
                            }
                        };
                        if created_channels.is_empty() {
                            create_channel_and_move_user.await;
                        } else {
                            // user already created a voice channel before, move the user there.
                            if let Err(err) = member
                                .move_to_voice_channel(
                                    &cx,
                                    ChannelId::new(created_channels[0].id.try_into().unwrap()),
                                )
                                .await {
                                    match err {
                                        serenity::Error::Http(serenity::all::HttpError::UnsuccessfulRequest(err)) => {
                                            if err.error.code == 10003 {
                                                // the user's channel no longer exists.
                                                delete(voice_channels::table)
                                                    .filter(voice_channels::id.eq(created_channels[0].id))
                                                    .execute(&mut conn)
                                                    .expect("Unable to delete user's voice channel record.");
                                                create_channel_and_move_user.await;
                                            }
                                        },
                                        _ => {}
                                    }
                                }
                        }
                    }
                }
            } else {
                let guild_id = new.guild_id.unwrap();
                let results = voice_channels::table
                    .filter(
                        voice_channels::guild.eq(TryInto::<i64>::try_into(guild_id.get()).unwrap()),
                    )
                    .select(VoiceChannel::as_select())
                    .load(&mut conn)
                    .unwrap();
                let guild_channels = guild_id.channels(&cx).await.unwrap();
                for voice_channel in results {
                    if let Some(channel) =
                        guild_channels.get(&ChannelId::new(voice_channel.id.try_into().unwrap()))
                    {
                        if channel.members(&cx).unwrap().len() == 0 {
                            channel
                                .delete(&cx)
                                .await
                                .expect("Unable to delete empty temporary voice channel.");
                        }
                    }
                }
            }
    }

    async fn interaction_create(&self, cx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Component(interaction) => {
                let id = interaction.data.custom_id.clone();
                if id == "voice_kick_user".to_string() {
                    let lck = cx.data.read().await;
                    let mut conn = lck.get::<ConnectionPoolKey>().unwrap().get().unwrap();
                    let msg = interaction.message.clone();
                    let channel_id = msg.channel_id;
                    let results = voice_channels::table
                        .filter(
                            voice_channels::id
                                .eq(TryInto::<i64>::try_into(channel_id.get()).unwrap()),
                        )
                        .filter(
                            voice_channels::creator
                                .eq(TryInto::<i64>::try_into(interaction.user.id.get()).unwrap()),
                        )
                        .select(VoiceChannel::as_select())
                        .load(&mut conn)
                        .unwrap();
                    if results.is_empty() {
                        // this is quite impossible, is it really necessary?
                        interaction
                            .create_response(
                                &cx,
                                CreateInteractionResponse::UpdateMessage(
                                    CreateInteractionResponseMessage::new()
                                        .content("Invalid channel")
                                        .embeds(vec![])
                                        .components(vec![]),
                                ),
                            )
                            .await
                            .unwrap();
                    } else {
                        match interaction.data.kind {
                            ComponentInteractionDataKind::StringSelect { ref values } => {
                                let mut members = msg
                                    .channel_id
                                    .to_channel(&cx)
                                    .await
                                    .unwrap()
                                    .guild()
                                    .unwrap()
                                    .members(&cx)
                                    .unwrap();
                                let mut kicked_users = vec![];
                                for val in values {
                                    if let Some(member) = members
                                        .iter_mut()
                                        .find(|x| x.user.id.get().to_string() == *val)
                                    {
                                        kicked_users.push(format!("<@{}>", member.user.id.get()));
                                        if let Err(err) = member.disconnect_from_voice(&cx).await {
                                            println!(
                                                "Failed to kick user from temp voice: {err:?}"
                                            );
                                        }
                                    }
                                }

                                interaction
                                    .create_response(
                                        &cx,
                                        CreateInteractionResponse::UpdateMessage(
                                            CreateInteractionResponseMessage::new()
                                                .content(if !kicked_users.is_empty() {
                                                    "The selected user has been kicked out."
                                                } else {
                                                    "No valid user to be kicked."
                                                })
                                                .embeds(vec![])
                                                .components(vec![]),
                                        ),
                                    )
                                    .await
                                    .unwrap();
                                interaction
                                    .create_followup(
                                        &cx,
                                        CreateInteractionResponseFollowup::new().content(format!(
                                            "The owner of this channel kicked {} out.",
                                            kicked_users.join(" , ")
                                        )),
                                    )
                                    .await
                                    .unwrap();
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
