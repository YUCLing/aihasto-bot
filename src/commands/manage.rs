use diesel::{delete, insert_into, ExpressionMethods, RunQueryDsl};
use serenity::all::{ChannelId, Role};

use crate::{
    models::{allowed_role::CreateAllowedRole, guild_settings::GuildSettings},
    schema::allowed_roles,
    Context, Error,
};

#[poise::command(
    slash_command,
    guild_only,
    subcommands(
        "sman_tempvoice",
        "sman_set_moderation_log_channel",
        "sman_allowed_roles"
    ),
    required_permissions = "ADMINISTRATOR"
)]
pub async fn sman(_cx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    rename = "tempvoice",
    subcommands("sman_tempvoice_set_channel")
)]
pub async fn sman_tempvoice(_cx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Set the creator channel for temporary voice.
#[poise::command(slash_command, guild_only, ephemeral, rename = "set_channel")]
pub async fn sman_tempvoice_set_channel(
    cx: Context<'_>,
    #[description = "The channel that will be the creator channel, ignore this to disable."]
    #[channel_types("Voice")]
    channel: Option<ChannelId>,
) -> Result<(), Error> {
    let guild = cx.guild_id().unwrap();
    let mut conn = cx.data().database.get()?;
    if let Some(channel) = channel {
        GuildSettings::set(
            &mut conn,
            guild,
            "creator_voice_channel",
            Some(channel.get().to_string()),
        )?;
        cx.say(format!(
            "The creator voice channel has been set to <#{}>",
            channel.get()
        ))
        .await?;
    } else {
        GuildSettings::set(&mut conn, guild, "creator_voice_channel", None::<String>)?;
        cx.say("The temporary voice channel creation has been disabled.")
            .await?;
    }
    Ok(())
}

/// Set the moderation log channel.
#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    rename = "set_moderation_log_channel"
)]
pub async fn sman_set_moderation_log_channel(
    cx: Context<'_>,
    #[description = "The channel that will be the moderation log channel, ignore to disable"]
    #[channel_types("Text")]
    channel: Option<ChannelId>,
) -> Result<(), Error> {
    let guild = cx.guild_id().unwrap();
    let mut conn = cx.data().database.get()?;
    if let Some(channel) = channel {
        GuildSettings::set(
            &mut conn,
            guild,
            "moderation_log_channel",
            Some(channel.get().to_string()),
        )?;
        cx.say(format!(
            "The moderation log channel has been set to <#{}>",
            channel.get()
        ))
        .await?;
    } else {
        GuildSettings::set(&mut conn, guild, "moderation_log_channel", None::<String>)?;
        cx.say("The moderation log channel creation has been disabled.")
            .await?;
    }
    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    rename = "allowed_roles",
    subcommands("sman_allowed_roles_add", "sman_allowed_roles_remove")
)]
pub async fn sman_allowed_roles(_cx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add an allowed role to a role
///
/// This makes the role can assign or remove the role.
#[poise::command(slash_command, guild_only, ephemeral, rename = "add")]
pub async fn sman_allowed_roles_add(
    cx: Context<'_>,
    #[description = "Operator role"] operator: Role,
    #[description = "Target role"] role: Role,
) -> Result<(), Error> {
    let operator_id = operator.id.get();
    let role_id = role.id.get();
    let mut conn = cx.data().database.get()?;
    insert_into(allowed_roles::table)
        .values(&[CreateAllowedRole::new(
            cx.guild_id().unwrap(),
            operator,
            role,
        )])
        .on_conflict_do_nothing()
        .execute(&mut conn)?;
    cx.say(format!(
        "You have allowed <@&{}> to assign/remove <@&{}> to other users.",
        operator_id, role_id
    ))
    .await?;
    Ok(())
}

/// Remove an allowed role from a role
///
/// This makes the role can no longer assign or remove the role.
#[poise::command(slash_command, guild_only, ephemeral, rename = "remove")]
pub async fn sman_allowed_roles_remove(
    cx: Context<'_>,
    #[description = "Operator role"] operator: Role,
    #[description = "Target role"] role: Role,
) -> Result<(), Error> {
    let operator_id = operator.id.get();
    let role_id = role.id.get();
    let mut conn = cx.data().database.get()?;
    let count = delete(allowed_roles::table)
        .filter(allowed_roles::role_id.eq(TryInto::<i64>::try_into(role.id.get()).unwrap()))
        .filter(
            allowed_roles::operator_role.eq(TryInto::<i64>::try_into(operator.id.get()).unwrap()),
        )
        .execute(&mut conn)?;
    cx.say(if count > 0 {
        format!(
            "You have disallowed <@&{}> to assign/remove <@&{}> to other users.",
            operator_id, role_id
        )
    } else {
        format!(
            "You didn't allow <@&{}> to assign/remove <@&{}> to other users before.",
            operator_id, role_id
        )
    })
    .await?;
    Ok(())
}
