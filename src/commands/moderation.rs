use std::str::FromStr;

use diesel::{
    update, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SelectableHelper,
};
use poise::{Context as PoiseContext, CreateReply};
use serenity::all::{
    ChannelId, Colour, CreateActionRow, CreateEmbed, CreateInputText, CreateInteractionResponse,
    CreateMessage, CreateModal, EditChannel, EditMessage, Member, MessageId, User,
};
use uuid::Uuid;

use crate::{
    features::{moderation::flood_impl, moderation_dm::generate_dm_message},
    models::{
        guild_settings::GuildSettings,
        moderation_log::{CreateModerationLog, ModerationAction, ModerationLog},
    },
    util::{parse_duration_to_seconds, send_moderation_logs_with_database_records},
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
    let actor = cx.author();
    let reason = format!("Rate limit updated by @{} ({})", actor.name, actor.id);
    channel
        .edit(
            cx,
            EditChannel::new()
                .rate_limit_per_user(cooldown)
                .audit_log_reason(&reason),
        )
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
    for result in {
        use crate::schema::moderation_log::*;
        use diesel::dsl::*;
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
            ModerationAction::Ban => bans = result.1,
        }
    }
    let logs: Vec<CreateEmbed> = ModerationLog::all()
        .filter(ModerationLog::by_user(user.clone()))
        .order_by(crate::schema::moderation_log::created_at.desc())
        .limit(5)
        .load::<ModerationLog>(&mut conn)?
        .into_iter()
        .map(|x| x.into())
        .collect();
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
            logs,
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
    cx.say(format!("The user has been warned.\nCase ID: `{}`", uuid))
        .await?;
    if let Some(channel) = GuildSettings::get(&mut conn, guild_id, "moderation_log_channel") {
        send_moderation_logs_with_database_records(
            &mut conn,
            &cx,
            guild_id,
            ChannelId::new(channel.parse().unwrap()),
            [log],
        )
        .await?;
    }
    Ok(())
}

/// Make a user Flooder. Use in the channel where the user violates the rules.
#[poise::command(slash_command, ephemeral, default_member_permissions = "MUTE_MEMBERS")]
pub async fn flood(
    cx: Context<'_>,
    #[description = "User that gets the Flooder"] user: Member,
    #[description = "The duration that user will be the Flooder"] duration: String,
    #[description = "Reason of making the user a Flooder"] reason: Option<String>,
) -> Result<(), Error> {
    let mut conn = cx.data().database.get()?;
    let queue = &cx.data().queue;
    cx.say(
        flood_impl(
            &cx,
            (&mut conn, queue),
            cx.channel_id(),
            user,
            cx.author(),
            duration,
            reason,
        )
        .await?,
    )
    .await?;
    Ok(())
}

#[poise::command(context_menu_command = "Flood", ephemeral)]
pub async fn flood_with_interaction(cx: Context<'_>, user: User) -> Result<(), Error> {
    if let PoiseContext::Application(cx) = cx {
        cx.interaction
            .create_response(
                &cx,
                CreateInteractionResponse::Modal(
                    CreateModal::new(
                        format!("flood:{}", user.id),
                        format!("Flood @{}", user.name),
                    )
                    .components(vec![
                        CreateActionRow::InputText(
                            CreateInputText::new(
                                serenity::all::InputTextStyle::Short,
                                "Reason",
                                "reason",
                            )
                            .required(false)
                            .placeholder("Leave blank for no reason"),
                        ),
                        CreateActionRow::InputText(
                            CreateInputText::new(
                                serenity::all::InputTextStyle::Short,
                                "Duration",
                                "duration",
                            )
                            .placeholder("e.g. 2h30m"),
                        ),
                    ]),
                ),
            )
            .await?;
    }
    Ok(())
}

/// Update the reason of a case.
#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    default_member_permissions = "MUTE_MEMBERS"
)]
pub async fn reason(
    cx: Context<'_>,
    #[description = "ID of the case to be updated"]
    #[rename = "id"]
    case_id: String,
    #[description = "New reason"]
    #[rename = "reason"]
    new_reason: String,
) -> Result<(), Error> {
    use crate::schema::moderation_log::*;
    let mut conn = cx.data().database.get()?;
    let Some(log) = update(table)
        .filter(id.eq(Uuid::from_str(&case_id).map_err(|_| "Case ID is invalid.")?))
        .set((reason.eq(new_reason), updated_at.eq(diesel::dsl::now)))
        .returning(ModerationLog::as_returning())
        .get_result(&mut conn)
        .optional()?
    else {
        cx.say("No case with provided ID found.").await?;
        return Ok(());
    };
    if let Some(channel) =
        GuildSettings::get(&mut conn, cx.guild_id().unwrap(), "moderation_log_channel")
    {
        let result: Option<(i64, i64, i64)> = {
            use crate::schema::moderation_log_message::*;
            table
                .filter(log_id.eq(log.id))
                .select((id, guild, channel))
                .get_result(&mut conn)
                .optional()?
        };
        let channel = ChannelId::new(channel.parse().unwrap());
        channel
            .send_message(
                &cx,
                if let Some((message_id, guild_id, channel_id)) = result {
                    channel.edit_message(&cx, MessageId::new(message_id.try_into().unwrap()), EditMessage::new()
                        .embed(log.into()))
                        .await?;
                    CreateMessage::new().content(format!(
                        "A case has been updated.\nLink to the case: https://discord.com/channels/{}/{}/{}",
                        guild_id, channel_id, message_id
                    ))
                } else {
                    CreateMessage::new()
                        .content("A case has been updated.")
                        .embed(log.into())
                },
            )
            .await?;
    }
    cx.say("Case has been updated.").await?;
    Ok(())
}
