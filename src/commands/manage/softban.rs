use serenity::all::{PermissionOverwrite, PermissionOverwriteType, Permissions, RoleId};

use crate::{
    commands::manage::set_server_id_impl, models::guild_settings::GuildSettings, Context, Error,
};

#[poise::command(slash_command, subcommands("setup_permissions", "set_role"))]
pub async fn softban(_cx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Block the softban role from using all channels in this server.
#[poise::command(slash_command, ephemeral)]
pub async fn setup_permissions(cx: Context<'_>) -> Result<(), Error> {
    let guild_id = cx.guild_id().unwrap();
    let Some(role) = GuildSettings::get(&mut cx.data().database.get()?, guild_id, "softban_role")
    else {
        return Ok(());
    };
    let role = RoleId::new(role.parse().unwrap());
    let channels = guild_id.channels(&cx).await?;
    let permission_override = PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::all(),
        kind: PermissionOverwriteType::Role(role),
    };
    for (_id, channel) in channels {
        if channel.parent_id.is_none() || !channel.permission_overwrites.is_empty() {
            // is a channel without parent, is a category or is not sync with the parent
            channel
                .create_permission(&cx, permission_override.clone())
                .await?;
        }
    }
    cx.say(format!("Denied <@&{}> from accessing all channels.", role))
        .await?;
    Ok(())
}

/// Set role for softban.
#[poise::command(slash_command, ephemeral)]
pub async fn set_role(
    cx: Context<'_>,
    #[description = "The new softban role, leave blank to disable."] role: Option<RoleId>,
) -> Result<(), Error> {
    cx.say(
        set_server_id_impl(
            "softban_role",
            "softban role",
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
