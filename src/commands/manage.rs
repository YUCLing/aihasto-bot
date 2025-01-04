use crate::{models::guild_settings::GuildSettings, Context, Error};

mod allowed_roles;
mod channels;
mod tempvoice;

use allowed_roles::allowed_roles as sman_allowed_roles;
use channels::channels as sman_channels;
use serenity::all::RoleId;
use tempvoice::tempvoice as sman_tempvoice;

#[poise::command(
    slash_command,
    guild_only,
    subcommands(
        "sman_tempvoice",
        "sman_channels",
        "sman_allowed_roles",
        "set_flooder_role"
    ),
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn sman(_cx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, ephemeral)]
pub async fn set_flooder_role(
    cx: Context<'_>,
    #[description = "Role that will be Flooder role, ignore to unset"] role: Option<RoleId>,
) -> Result<(), Error> {
    let guild = cx.guild_id().unwrap();
    let mut conn = cx.data().database.get()?;
    if let Some(role) = role {
        GuildSettings::set(
            &mut conn,
            guild,
            "flooder_role",
            Some(role.get().to_string()),
        )?;
        cx.say(format!(
            "The Flooder role has been set to <@&{}>",
            role.get()
        ))
        .await?;
    } else {
        GuildSettings::set(&mut conn, guild, "flooder_role", None::<String>)?;
        cx.say("The Flooder role has been disabled.").await?;
    }
    Ok(())
}
