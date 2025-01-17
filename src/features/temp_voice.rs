use diesel::{
    dsl::{delete, insert_into},
    query_dsl::methods::{FilterDsl, LimitDsl, SelectDsl},
    ExpressionMethods, RunQueryDsl, SelectableHelper,
};
use serenity::all::{
    CacheHttp, ChannelId, ComponentInteractionDataKind, Context, CreateChannel,
    CreateInteractionResponse, CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
    CreateMessage, EditMember, GuildChannel, GuildId, Interaction, Member, User, VoiceState,
};

use crate::{
    data::ConnectionPoolKey,
    models::{
        guild_settings::GuildSettings,
        voice_channel::{CreateVoiceChannel, VoiceChannel},
    },
    schema::voice_channels,
    util::get_conn_from_serenity,
    Connection, Error,
};

pub fn default_channel_name_for_member(member: &Member) -> String {
    member
        .nick
        .clone()
        .unwrap_or(member.user.display_name().to_string())
}

pub async fn create_temp_voice_channel<U: CacheHttp, V: Into<GuildId>, W: AsRef<str>>(
    conn: &mut Connection,
    http: &U,
    guild: V,
    creator: &User,
    name: W,
    category: Option<ChannelId>,
) -> Result<GuildChannel, Error> {
    let reason = format!("Created by @{} ({})", creator.name, creator.id.get());
    let mut create_channel = CreateChannel::new(name.as_ref().to_string())
        .kind(serenity::all::ChannelType::Voice)
        .audit_log_reason(&reason);
    if let Some(category) = category {
        create_channel = create_channel.category(category);
    }
    let guild = guild.into();
    let channel = guild.create_channel(http, create_channel).await;
    match channel {
        Ok(channel) => {
            insert_into(voice_channels::table)
                .values(&[CreateVoiceChannel::new(&channel, guild, creator)])
                .execute(conn)?;
            Ok(channel)
        }
        Err(err) => Err(Box::new(err)),
    }
}

pub async fn handle_voice_state_update(cx: Context, new: VoiceState) {
    let pool = cx
        .data
        .read()
        .await
        .get::<ConnectionPoolKey>()
        .unwrap()
        .clone();
    if let Some(channel_id) = new.channel_id {
        let guild_id = new.guild_id.unwrap();
        let Some(guild_voice_creator) =
            GuildSettings::get(&mut pool.get().unwrap(), guild_id, "creator_voice_channel")
        else {
            return;
        };
        if channel_id.get().to_string() == guild_voice_creator {
            if let Some(member) = &new.member {
                let created_channels = voice_channels::table
                    .filter(
                        voice_channels::creator
                            .eq(TryInto::<i64>::try_into(member.user.id.get()).unwrap()),
                    )
                    .limit(1)
                    .select(VoiceChannel::as_select())
                    .load(&mut pool.get().unwrap())
                    .unwrap();
                let create_channel_and_move_user = async {
                    let parent_channel = channel_id
                        .to_channel(&cx)
                        .await
                        .unwrap()
                        .guild()
                        .unwrap()
                        .parent_id;
                    match create_temp_voice_channel(
                        &mut pool.get().unwrap(),
                        &cx,
                        &guild_id,
                        &member.user,
                        default_channel_name_for_member(member),
                        parent_channel,
                    )
                    .await
                    {
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
                        }
                        Err(err) => {
                            log::warn!("Failed to create voice channel: {err:?}");
                        }
                    }
                };
                if created_channels.is_empty() {
                    create_channel_and_move_user.await;
                } else {
                    // user already created a voice channel before, move the user there.
                    if let Err(serenity::Error::Http(
                        serenity::all::HttpError::UnsuccessfulRequest(err),
                    )) = member
                        .move_to_voice_channel(
                            &cx,
                            ChannelId::new(created_channels[0].id.try_into().unwrap()),
                        )
                        .await
                    {
                        if err.error.code == 10003 {
                            // the user's channel no longer exists.
                            delete(voice_channels::table)
                                .filter(voice_channels::id.eq(created_channels[0].id))
                                .execute(&mut pool.get().unwrap())
                                .expect("Unable to delete user's voice channel record.");
                            create_channel_and_move_user.await;
                        }
                    }
                }
            }
        }
    } else {
        // a user is leaving the voice channel, let's clean up all temporary channels in this guild.
        let guild_id = new.guild_id.unwrap();
        let results = voice_channels::table
            .filter(voice_channels::guild.eq(TryInto::<i64>::try_into(guild_id.get()).unwrap()))
            .select(VoiceChannel::as_select())
            .load(&mut pool.get().unwrap())
            .unwrap();
        let guild_channels = guild_id.channels(&cx).await.unwrap();
        for voice_channel in results {
            if let Some(channel) =
                guild_channels.get(&ChannelId::new(voice_channel.id.try_into().unwrap()))
            {
                if channel.members(&cx).unwrap().is_empty() {
                    channel
                        .delete(&cx)
                        .await
                        .expect("Unable to delete empty temporary voice channel.");
                }
            }
        }
    }
}

pub async fn handle_interaction(cx: Context, interaction: Interaction) {
    if let Interaction::Component(interaction) = interaction {
        let id = interaction.data.custom_id.clone();
        if id == *"voice_kick_user" {
            let msg = interaction.message.clone();
            let channel_id = msg.channel_id;
            let results = voice_channels::table
                .filter(voice_channels::id.eq(TryInto::<i64>::try_into(channel_id.get()).unwrap()))
                .filter(
                    voice_channels::creator
                        .eq(TryInto::<i64>::try_into(interaction.user.id.get()).unwrap()),
                )
                .select(VoiceChannel::as_select())
                .load(&mut get_conn_from_serenity(&cx).await.unwrap())
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
            } else if let ComponentInteractionDataKind::StringSelect { ref values } =
                interaction.data.kind
            {
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
                let actor = interaction.user.clone();
                let reason = format!("Kicked by @{} ({})", actor.name, actor.id);
                for val in values {
                    if let Some(member) = members
                        .iter_mut()
                        .find(|x| x.user.id.get().to_string() == *val)
                    {
                        kicked_users.push(format!("<@{}>", member.user.id.get()));
                        if let Err(err) = member
                            .edit(
                                &cx,
                                EditMember::new()
                                    .disconnect_member()
                                    .audit_log_reason(&reason),
                            )
                            .await
                        {
                            log::warn!("Failed to kick user from temp voice: {err:?}");
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
        }
    }
}
