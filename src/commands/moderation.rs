use poise::Context as PoiseContext;
use serenity::all::{
    CreateActionRow, CreateInputText, CreateInteractionResponse, CreateModal, EditChannel, Member,
    RoleId, User,
};

use crate::{
    features::moderation::{flood_impl, inspect_impl, warning_impl},
    models::guild_settings::GuildSettings,
    util::parse_duration_to_seconds,
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
    context_menu_command = "Inspect",
    guild_only,
    ephemeral,
    default_member_permissions = "MUTE_MEMBERS"
)]
pub async fn context_menu_inspect(cx: Context<'_>, user: User) -> Result<(), Error> {
    cx.send(inspect_impl::<&str>(&cx.data().database, user, None).await?)
        .await?;
    Ok(())
}

/// Inspect a user
#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    default_member_permissions = "MUTE_MEMBERS"
)]
pub async fn inspect(
    cx: Context<'_>,
    #[description = "The user to be inspected"] user: User,
    #[description = "Filter of the moderation kind"] filter: Option<String>,
) -> Result<(), Error> {
    cx.send(inspect_impl(&cx.data().database, user, filter).await?)
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
    cx.say(
        warning_impl(
            &cx,
            &cx.data().database,
            cx.channel_id(),
            user,
            cx.author(),
            reason,
        )
        .await?,
    )
    .await?;
    Ok(())
}

#[poise::command(
    context_menu_command = "Warning",
    guild_only,
    ephemeral,
    default_member_permissions = "MUTE_MEMBERS"
)]
pub async fn warning_with_interaction(cx: Context<'_>, user: User) -> Result<(), Error> {
    if let PoiseContext::Application(cx) = cx {
        cx.interaction
            .create_response(
                &cx,
                CreateInteractionResponse::Modal(
                    CreateModal::new(
                        format!("warning:{}", user.id),
                        format!("Warning @{}", user.name),
                    )
                    .components(vec![CreateActionRow::InputText(
                        CreateInputText::new(
                            serenity::all::InputTextStyle::Short,
                            "Reason",
                            "reason",
                        )
                        .required(false)
                        .placeholder("Leave blank for no reason"),
                    )]),
                ),
            )
            .await?;
    }
    Ok(())
}

/// Make a user Flooder. Use in the channel where the user violates the rules.
#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    default_member_permissions = "MUTE_MEMBERS"
)]
pub async fn flood(
    cx: Context<'_>,
    #[description = "User that gets the Flooder"] user: Member,
    #[description = "The duration that user will be the Flooder"] duration: String,
    #[description = "Reason of making the user a Flooder"] reason: Option<String>,
) -> Result<(), Error> {
    let queue = &cx.data().queue;
    cx.say(
        flood_impl(
            &cx,
            (&cx.data().database, queue),
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

#[poise::command(
    context_menu_command = "Flood",
    guild_only,
    ephemeral,
    default_member_permissions = "MUTE_MEMBERS"
)]
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

/// Make a user no longer a Flooder
#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    default_member_permissions = "MUTE_MEMBERS"
)]
pub async fn unflood(
    cx: Context<'_>,
    #[description = "The user that will be unflooded."] user: Member,
) -> Result<(), Error> {
    let Some(flooder_role) =
        GuildSettings::get(&cx.data().database, cx.guild_id().unwrap(), "flooder_role")
            .map(|x| RoleId::new(x.parse().unwrap()))
    else {
        cx.say("Flooder is disabled.").await?;
        return Ok(());
    };
    user.remove_role(cx, flooder_role).await?;
    cx.say("Removed the Flooder role from the user.").await?;
    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    default_member_permissions = "MUTE_MEMBERS"
)]
pub async fn softban(_cx: Context<'_>) -> Result<(), Error> {
    todo!("implement softban")
}

#[poise::command(
    context_menu_command = "Softban",
    ephemeral,
    default_member_permissions = "MUTE_MEMBERS"
)]
pub async fn softban_with_interaction(cx: Context<'_>, user: User) -> Result<(), Error> {
    if let PoiseContext::Application(cx) = cx {
        cx.interaction
            .create_response(
                &cx,
                CreateInteractionResponse::Modal(
                    CreateModal::new(
                        format!("softban:{}", user.id),
                        format!("Softban @{}", user.name),
                    )
                    .components(vec![CreateActionRow::InputText(
                        CreateInputText::new(
                            serenity::all::InputTextStyle::Short,
                            "Reason",
                            "reason",
                        )
                        .required(false)
                        .placeholder("Leave blank for no reason"),
                    )]),
                ),
            )
            .await?;
    }
    Ok(())
}
