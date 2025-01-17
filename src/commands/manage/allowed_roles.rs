use diesel::{
    dsl::{delete, insert_into},
    ExpressionMethods, RunQueryDsl,
};
use serenity::all::Role;

use crate::{models::allowed_role::CreateAllowedRole, schema::allowed_roles, Context, Error};

#[poise::command(slash_command, guild_only, subcommands("add", "remove"))]
pub async fn allowed_roles(_cx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add an allowed role to a role
///
/// This makes the role can assign or remove the role.
#[poise::command(slash_command, guild_only, ephemeral)]
pub async fn add(
    cx: Context<'_>,
    #[description = "Operator role"] operator: Role,
    #[description = "Target role"] role: Role,
) -> Result<(), Error> {
    let operator_id = operator.id.get();
    let role_id = role.id.get();
    insert_into(allowed_roles::table)
        .values(&[CreateAllowedRole::new(
            cx.guild_id().unwrap(),
            operator,
            role,
        )])
        .on_conflict_do_nothing()
        .execute(&mut cx.data().database.get()?)?;
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
#[poise::command(slash_command, guild_only, ephemeral)]
pub async fn remove(
    cx: Context<'_>,
    #[description = "Operator role"] operator: Role,
    #[description = "Target role"] role: Role,
) -> Result<(), Error> {
    let operator_id = operator.id.get();
    let role_id = role.id.get();
    let count = delete(allowed_roles::table)
        .filter(allowed_roles::role_id.eq(TryInto::<i64>::try_into(role.id.get()).unwrap()))
        .filter(
            allowed_roles::operator_role.eq(TryInto::<i64>::try_into(operator.id.get()).unwrap()),
        )
        .execute(&mut cx.data().database.get()?)?;
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
