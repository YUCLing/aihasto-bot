use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use fang::AsyncQueueable;
use poise::CreateReply;
use serenity::all::{ChannelId, Colour, CreateEmbed, EditChannel, Member, RoleId, User};

use crate::{
    features::{moderation_dm::generate_dm_message, temp_role::RemoveTempRole},
    models::{
        guild_settings::GuildSettings,
        moderation_log::{CreateModerationLog, ModerationAction, ModerationLog},
    },
    util::{parse_duration_to_seconds, send_moderation_logs},
    Context, Error,
};

/// Set or remove rate limit for a channel
///
/// Example Usage:
/// `/slowmode 2h` - Sets rate limit to 2 hours per message.
/// `/slowmode` - Removes rate limit
#[poise::command(
    slash_command,
    ephemeral,
    guild_only,
    category = "Moderation",
    required_bot_permissions = "MANAGE_CHANNELS",
    default_member_permissions = "MANAGE_CHANNELS"
)]
pub async fn slowmode(
    cx: Context<'_>,
    #[description = "Message cooldown in seconds (max 6 hours), leave empty or set to 0 to remove"]
    cooldown: Option<String>,
    #[description = "Channel to operate on, defaults to current channel"]
    #[channel_types("Text")]
    channel: Option<serenity::all::GuildChannel>,
) -> Result<(), Error> {
    let cooldown = match parse_duration_to_seconds(cooldown.unwrap_or("0".to_string()))
        .and_then(|x| x.try_into().map_err(|_| "Invalid number".to_string()))
    {
        Ok(x) => x,
        Err(err) => {
            cx.say(err).await?;
            return Ok(());
        }
    };
    cx.say(if cooldown == 0 {
        "Removing rate limit"
    } else {
        "Setting channel to slowmode"
    })
    .await?;
    let mut channel = channel.unwrap_or(cx.guild_channel().await.unwrap());
    channel
        .edit(cx, EditChannel::new().rate_limit_per_user(cooldown))
        .await?;
    Ok(())
}

/// Inspect a user
#[poise::command(
    slash_command,
    context_menu_command = "Inspect",
    ephemeral,
    default_member_permissions = "MUTE_MEMBERS"
)]
pub async fn inspect(
    cx: Context<'_>,
    #[description = "The user to be inspected"] user: User,
) -> Result<(), Error> {
    let mut conn = cx.data().database.get()?;
    let mut warns = 0;
    let mut floods = 0;
    let mut timeouts = 0;
    let mut bans = 0;
    let logs: Vec<ModerationLog> = ModerationLog::all()
        .filter(ModerationLog::by_user(user.clone()))
        .order_by(crate::schema::moderation_log::created_at.desc())
        .limit(5)
        .load(&mut conn)?;
    let mut embeds: Vec<CreateEmbed> = vec![];
    for log in logs {
        match log.kind {
            ModerationAction::Warning => warns += 1,
            ModerationAction::Flood => floods += 1,
            ModerationAction::Timeout => timeouts += 1,
            ModerationAction::Ban => bans += 1,
        }
        embeds.push(log.into());
    }
    cx.send(CreateReply {
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
                        ModerationAction::Ban.embed_title(),
                        format!("{} time(s)", bans),
                        true,
                    ),
                ])],
            embeds,
        ]
        .concat(),
        ..Default::default()
    })
    .await?;
    Ok(())
}

/// Warn a user. Use in the channel where the user violates the rules.
#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    default_member_permissions = "MUTE_MEMBERS"
)]
pub async fn warning(
    cx: Context<'_>,
    #[description = "The user that receives the warning"] user: Member,
    #[description = "Reason of warning"] reason: Option<String>,
) -> Result<(), Error> {
    let mut conn = cx.data().database.get()?;
    let guild_id = cx.guild().unwrap().id;
    let log: ModerationLog = ModerationLog::insert()
        .values([CreateModerationLog::new(
            guild_id,
            ModerationAction::Warning,
            user.user.id,
            Some(cx.author().id),
            reason.clone(),
        )])
        .get_result(&mut conn)?;
    let uuid = log.id;
    user.user
        .dm(
            &cx,
            generate_dm_message(&log, cx.author(), Some(cx.channel_id())),
        )
        .await?;
    cx.say(format!(
        "The user has been warned.\nCase ID: `{}`",
        uuid.to_string()
    ))
    .await?;
    if let Some(channel) = GuildSettings::get(&mut conn, guild_id, "moderation_log_channel") {
        send_moderation_logs(&cx, ChannelId::new(channel.parse().unwrap()), [log]).await?;
    }
    Ok(())
}

/// Make a user Flooder. Use in the channel where the user violates the rules.
#[poise::command(slash_command, ephemeral, default_member_permissions = "MUTE_MEMBERS")]
pub async fn flood(
    cx: Context<'_>,
    #[description = "User that gets the Flooder"] user: Member,
    #[description = "The duration that user will be the Flooder"] mut duration: String,
    #[description = "Reason of making the user a Flooder"] reason: Option<String>,
) -> Result<(), Error> {
    let duration_secs = match parse_duration_to_seconds(&duration)
        .and_then(|x| x.try_into().map_err(|_| "Invalid number".to_string()))
    {
        Ok(x) => x,
        Err(err) => {
            cx.say(err).await?;
            return Ok(());
        }
    };
    if duration_secs <= 0 {
        cx.say("Invalid duration").await?;
        return Ok(());
    }
    let guild_id = cx.guild_id().unwrap();
    let mut conn = cx.data().database.get()?;
    let Some(flooder_role) = GuildSettings::get(&mut conn, guild_id, "flooder_role")
        .and_then(|x| Some(RoleId::new(x.parse().unwrap())))
    else {
        cx.say("Flooder is disabled.").await?;
        return Ok(());
    };
    if user.roles.contains(&flooder_role) {
        cx.say("User is already a Flooder.").await?;
        return Ok(());
    }
    let queue = cx.data().queue.clone();
    let task = RemoveTempRole::new(guild_id, user.user.id, flooder_role, duration_secs);
    queue.schedule_task(&task).await?;
    if duration.chars().last().map_or(false, |c| c.is_numeric()) {
        duration.push('s');
    }
    let member = cx.author_member().await.unwrap();
    cx.http()
        .add_member_role(
            guild_id,
            user.user.id,
            flooder_role,
            Some(
                format!(
                    "Flooded by @{} ({}) with a duration of {}",
                    member.user.name, member.user.id, duration
                )
                .as_ref(),
            ),
        )
        .await?;
    let log: ModerationLog = ModerationLog::insert()
        .values([CreateModerationLog::new(
            guild_id,
            ModerationAction::Flood,
            user.user.id,
            Some(cx.author().id),
            reason.clone(),
        )])
        .get_result(&mut conn)?;
    let uuid = log.id;
    user.user
        .dm(
            &cx,
            generate_dm_message(&log, cx.author(), Some(cx.channel_id())),
        )
        .await?;
    cx.say(format!(
        "Made <@{}> Flooder with a duration of **{}**.\nCase ID: `{}`",
        user.user.id.get(),
        duration,
        uuid
    ))
    .await?;
    if let Some(channel) = GuildSettings::get(&mut conn, guild_id, "moderation_log_channel") {
        send_moderation_logs(&cx, ChannelId::new(channel.parse().unwrap()), [log]).await?;
    }
    Ok(())
}
