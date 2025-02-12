use diesel::RunQueryDsl;
use fang::{AsyncQueue, AsyncQueueable};
use poise::CreateReply;
use serenity::all::{
    ActionRowComponent, CacheHttp, ChannelId, Colour, Context, CreateEmbed,
    CreateInteractionResponse, CreateInteractionResponseMessage, Interaction, Member, RoleId, User,
    UserId,
};

use crate::{
    data::QueueKey,
    models::{
        guild_settings::GuildSettings,
        moderation_log::{CreateModerationLog, ModerationAction, ModerationLog},
    },
    util::{
        get_pool_from_serenity, parse_duration_to_seconds,
        send_moderation_logs_with_database_records,
    },
    ConnectionPool, Error,
};

use super::{
    moderation_dm::generate_dm_message, temp_role::RemoveTempRole, temp_warning::RemoveWarning,
};

pub async fn inspect_impl<F>(
    pool: &ConnectionPool,
    user: User,
    filter: Option<F>,
) -> Result<CreateReply, Error>
where
    F: AsRef<str>,
{
    let mut warns = 0;
    let mut floods = 0;
    let mut timeouts = 0;
    let mut softbans = 0;
    let mut bans = 0;
    let logs: Vec<CreateEmbed> = {
        use diesel::dsl::*;
        use diesel::ExpressionMethods;
        use diesel::QueryDsl;
        let mut conn = pool.get()?;
        for result in {
            use crate::schema::moderation_log::*;
            table
                .filter(ModerationLog::by_user(user.clone()))
                .group_by(kind)
                .select((kind, count_star()))
                .load::<(ModerationAction, i64)>(&mut conn)?
        } {
            match result.0 {
                ModerationAction::Warning => warns = result.1,
                ModerationAction::Flood => floods = result.1,
                ModerationAction::Timeout => timeouts = result.1,
                ModerationAction::Softban => softbans = result.1,
                ModerationAction::Ban => bans = result.1,
            }
        }
        let mut query = ModerationLog::all().filter(ModerationLog::by_user(user.clone()));
        if let Some(filter) = filter {
            for str in filter.as_ref().split(",") {
                let Ok(action): Result<ModerationAction, _> = str.trim().to_lowercase().try_into()
                else {
                    return Ok(CreateReply {
                        content: Some(format!("Unknown filter: {}", str)),
                        ..Default::default()
                    });
                };
                query = query.filter(crate::schema::moderation_log::kind.eq(action));
            }
        }
        query
            .order_by(crate::schema::moderation_log::created_at.desc())
            .limit(5)
            .load::<ModerationLog>(&mut conn)?
            .into_iter()
            .map(|x| x.into())
            .collect()
    };
    Ok(CreateReply {
        content: Some(format!("Moderation logs for <@{}>", user.id.get())),
        embeds: [
            vec![CreateEmbed::new()
                .title("Summary of moderations")
                .color(Colour::BLUE)
                .fields([
                    (
                        ModerationAction::Warning.embed_title(),
                        format!("{} time(s)", warns),
                        true,
                    ),
                    (
                        ModerationAction::Flood.embed_title(),
                        format!("{} time(s)", floods),
                        true,
                    ),
                    (
                        ModerationAction::Timeout.embed_title(),
                        format!("{} time(s)", timeouts),
                        true,
                    ),
                    (
                        ModerationAction::Softban.embed_title(),
                        format!("{} time(s)", softbans),
                        true,
                    ),
                    (
                        ModerationAction::Ban.embed_title(),
                        format!("{} time(s)", bans),
                        true,
                    ),
                ])],
            logs,
        ]
        .concat(),
        ..Default::default()
    })
}

pub async fn warning_impl<T: CacheHttp>(
    cx: &T,
    state: (&ConnectionPool, &AsyncQueue),
    channel: ChannelId,
    member: Member,
    actor: &User,
    reason: Option<String>,
    duration: Option<String>,
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
        .get_result(&mut state.0.get()?)?;
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
    if let Some(mut duration) = duration {
        let duration_secs = match parse_duration_to_seconds(&duration) {
            Ok(x) => x,
            Err(err) => {
                return Ok(err);
            }
        };
        if duration_secs == 0 {
            return Ok("Invalid duration".to_string());
        }
        if duration.chars().last().is_some_and(|c| c.is_numeric()) {
            duration.push('s');
        }
        let task = RemoveWarning::new(uuid, duration_secs);
        state.1.schedule_task(&task).await?;
        return Ok(format!(
            "The user has been warned with a duration of **{}**.\nCase ID: `{}`",
            duration, uuid
        ));
    }
    Ok(format!("The user has been warned.\nCase ID: `{}`", uuid))
}

pub async fn flood_impl<T: CacheHttp>(
    cx: &T,
    state: (&ConnectionPool, &AsyncQueue),
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
        .get_result(&mut state.0.get()?)?;
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

pub async fn softban_impl<T: CacheHttp>(
    cx: &T,
    pool: &ConnectionPool,
    channel: ChannelId,
    member: Member,
    actor: &User,
    reason: Option<String>,
) -> Result<String, Error> {
    let guild_id = member.guild_id;
    let Some(softban_role) =
        GuildSettings::get(pool, guild_id, "softban_role").map(|x| RoleId::new(x.parse().unwrap()))
    else {
        return Ok("Softban is disabled.".to_string());
    };
    let log: ModerationLog = ModerationLog::insert()
        .values([CreateModerationLog::new(
            guild_id,
            ModerationAction::Softban,
            member.user.id,
            Some(actor.id),
            reason.clone(),
        )])
        .get_result(&mut pool.get()?)?;
    cx.http()
        .add_member_role(
            guild_id,
            member.user.id,
            softban_role,
            Some(&format!("Softbanned by @{} ({})", actor.name, actor.id)),
        )
        .await?;
    let uuid = log.id;
    member
        .user
        .dm(&cx, generate_dm_message(&log, actor, Some(channel)))
        .await?;
    if let Some(channel) = GuildSettings::get(pool, guild_id, "moderation_log_channel") {
        send_moderation_logs_with_database_records(
            pool,
            &cx,
            guild_id,
            ChannelId::new(channel.parse().unwrap()),
            [log],
        )
        .await?;
    }
    Ok(format!(
        "The user has been softbanned.\nCase ID: `{}`",
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
            let mut duration = None;
            for row in &modal.data.components {
                for comp in &row.components {
                    if let ActionRowComponent::InputText(input) = comp {
                        if input.custom_id == "reason" {
                            let value = input.value.clone().unwrap();
                            if !value.is_empty() {
                                reason = Some(value);
                            }
                        } else if input.custom_id == "duration" {
                            let value = input.value.clone().unwrap();
                            if !value.is_empty() {
                                duration = Some(value);
                            }
                        }
                    }
                }
            }
            let res = warning_impl(
                &cx,
                (
                    &get_pool_from_serenity(&cx).await,
                    cx.data.read().await.get::<QueueKey>().unwrap(),
                ),
                modal.channel_id,
                member,
                &modal.user,
                reason,
                duration,
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
                (&get_pool_from_serenity(&cx).await, &queue),
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
