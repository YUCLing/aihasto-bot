use diesel::RunQueryDsl;
use fang::{AsyncQueue, AsyncQueueable};
use serenity::all::{
    ActionRowComponent, CacheHttp, ChannelId, Context, CreateInteractionResponse,
    CreateInteractionResponseMessage, Interaction, Member, RoleId, User, UserId,
};

use crate::{
    data::QueueKey,
    models::{
        guild_settings::GuildSettings,
        moderation_log::{CreateModerationLog, ModerationAction, ModerationLog},
    },
    util::{
        get_conn_from_serenity, parse_duration_to_seconds,
        send_moderation_logs_with_database_records,
    },
    Connection, Error,
};

use super::{moderation_dm::generate_dm_message, temp_role::RemoveTempRole};

pub async fn warning_impl<T: CacheHttp>(
    cx: &T,
    conn: &mut Connection,
    channel: ChannelId,
    member: Member,
    actor: &User,
    reason: Option<String>,
) -> Result<String, Error> {
    let guild_id = member.guild_id;
    let log: ModerationLog = ModerationLog::insert()
        .values([CreateModerationLog::new(
            guild_id,
            ModerationAction::Warning,
            member.user.id,
            Some(actor.id),
            reason.clone(),
        )])
        .get_result(conn)?;
    let uuid = log.id;
    member
        .user
        .dm(&cx, generate_dm_message(&log, actor, Some(channel)))
        .await?;
    if let Some(channel) = GuildSettings::get(conn, guild_id, "moderation_log_channel") {
        send_moderation_logs_with_database_records(
            conn,
            &cx,
            guild_id,
            ChannelId::new(channel.parse().unwrap()),
            [log],
        )
        .await?;
    }
    Ok(format!("The user has been warned.\nCase ID: `{}`", uuid))
}

pub async fn flood_impl<T: CacheHttp>(
    cx: &T,
    state: (&mut Connection, &AsyncQueue),
    channel: ChannelId,
    member: Member,
    actor: &User,
    mut duration: String,
    reason: Option<String>,
) -> Result<String, Error> {
    let duration_secs = match parse_duration_to_seconds(&duration) {
        Ok(x) => x,
        Err(err) => {
            return Ok(err);
        }
    };
    if duration_secs == 0 {
        return Ok("Invalid duration".to_string());
    }
    let guild_id = member.guild_id;
    let Some(flooder_role) = GuildSettings::get(state.0, guild_id, "flooder_role")
        .map(|x| RoleId::new(x.parse().unwrap()))
    else {
        return Ok("Flooder is disabled.".to_string());
    };
    if member.roles.contains(&flooder_role) {
        return Ok("User is already a Flooder.".to_string());
    }
    let task = RemoveTempRole::new(guild_id, member.user.id, flooder_role, duration_secs);
    state.1.schedule_task(&task).await?;
    if duration.chars().last().is_some_and(|c| c.is_numeric()) {
        duration.push('s');
    }
    cx.http()
        .add_member_role(
            guild_id,
            member.user.id,
            flooder_role,
            Some(
                format!(
                    "Flooded by @{} ({}) with a duration of {}",
                    actor.name, actor.id, duration
                )
                .as_ref(),
            ),
        )
        .await?;
    let log: ModerationLog = ModerationLog::insert()
        .values([CreateModerationLog::new(
            guild_id,
            ModerationAction::Flood,
            member.user.id,
            Some(actor.id),
            reason.clone(),
        )])
        .get_result(state.0)?;
    let uuid = log.id;
    member
        .user
        .dm(&cx, generate_dm_message(&log, actor, Some(channel)))
        .await?;
    if let Some(channel) = GuildSettings::get(state.0, guild_id, "moderation_log_channel") {
        send_moderation_logs_with_database_records(
            state.0,
            &cx,
            guild_id,
            ChannelId::new(channel.parse().unwrap()),
            [log],
        )
        .await?;
    }
    Ok(format!(
        "Made <@{}> Flooder with a duration of **{}**.\nCase ID: `{}`",
        member.user.id.get(),
        duration,
        uuid
    ))
}

pub async fn handle_interaction(cx: Context, interaction: Interaction) {
    if let Interaction::Modal(modal) = interaction {
        if let Some(id) = modal.data.custom_id.strip_prefix("warning:") {
            let user = UserId::new(id.parse().unwrap());
            let guild = modal.guild_id.unwrap();
            let member = guild.member(&cx, user).await.unwrap();
            let mut reason = None;
            for row in &modal.data.components {
                for comp in &row.components {
                    if let ActionRowComponent::InputText(input) = comp {
                        if input.custom_id == "reason" {
                            let value = input.value.clone().unwrap();
                            if !value.is_empty() {
                                reason = Some(value);
                            }
                        }
                    }
                }
            }
            let res = warning_impl(
                &cx,
                &mut get_conn_from_serenity(&cx).await.unwrap(),
                modal.channel_id,
                member,
                &modal.user,
                reason,
            )
            .await
            .unwrap();
            modal
                .create_response(
                    &cx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content(res),
                    ),
                )
                .await
                .unwrap();
        } else if let Some(id) = modal.data.custom_id.strip_prefix("flood:") {
            let queue = cx.data.read().await.get::<QueueKey>().unwrap().clone();
            let user = UserId::new(id.parse().unwrap());
            let guild = modal.guild_id.unwrap();
            let member = guild.member(&cx, user).await.unwrap();
            let mut duration = None;
            let mut reason = None;
            for row in &modal.data.components {
                for comp in &row.components {
                    if let ActionRowComponent::InputText(input) = comp {
                        match input.custom_id.as_ref() {
                            "reason" => {
                                let value = input.value.clone().unwrap();
                                if !value.is_empty() {
                                    reason = Some(value);
                                }
                            }
                            "duration" => duration = input.value.clone(),
                            _ => {}
                        }
                    }
                }
            }
            let res = flood_impl(
                &cx,
                (&mut get_conn_from_serenity(&cx).await.unwrap(), &queue),
                modal.channel_id,
                member,
                &modal.user,
                duration.unwrap(),
                reason,
            )
            .await
            .unwrap();
            modal
                .create_response(
                    &cx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content(res),
                    ),
                )
                .await
                .unwrap();
        }
    }
}
