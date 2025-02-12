use crate::{models::guild_settings::GuildSettings, ConnectionPool, Context, Error};
use serenity::all::{GuildId, RoleId};

mod allowed_roles;
mod channels;
mod softban;
mod tempvoice;

use allowed_roles::allowed_roles as sman_allowed_roles;
use channels::channels as sman_channels;
use softban::softban as sman_softban;
use tempvoice::tempvoice as sman_tempvoice;

#[poise::command(
    slash_command,
    guild_only,
    subcommands(
        "sman_tempvoice",
        "sman_channels",
        "sman_allowed_roles",
        "sman_softban",
        "set_flooder_role"
    ),
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn sman(_cx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

pub async fn set_server_id_impl<T: Into<u64>>(
    key: &str,
    name: &str,
    id_prefix: &str,
    pool: &ConnectionPool,
    guild: GuildId,
    id: Option<T>,
) -> Result<String, Error> {
    Ok(if let Some(id) = id {
        let id = id.into();
        GuildSettings::set(pool, guild, key, Some(id.to_string()))?;
        format!("The {} has been set to <{}{}>", name, id_prefix, id)
    } else {
        GuildSettings::set(pool, guild, key, None::<String>)?;
        format!("The {} has been disabled.", name)
    })
}

/// Set the Flooder role for the server.
#[poise::command(slash_command, ephemeral)]
pub async fn set_flooder_role(
    cx: Context<'_>,
    #[description = "Role that will be Flooder role, ignore to unset"] role: Option<RoleId>,
) -> Result<(), Error> {
    cx.say(
        set_server_id_impl(
            "flooder_role",
            "Flooder role",
            "@&",
            &cx.data().database,
            cx.guild_id().unwrap(),
            role,
        )
        .await?,
    )
    .await?;
    Ok(())
}
