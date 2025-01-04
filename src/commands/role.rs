use diesel::{
    query_dsl::methods::{FilterDsl, SelectDsl},
    ExpressionMethods, RunQueryDsl, SelectableHelper,
};
use fang::AsyncQueueable;
use serenity::all::{Member, RoleId};

use crate::{
    features::temp_role::RemoveTempRole, models::allowed_role::AllowedRole, schema::allowed_roles,
    util::parse_duration_to_seconds, Context, Error,
};

#[poise::command(
    slash_command,
    ephemeral,
    guild_only,
    subcommands("add", "remove"),
    default_member_permissions = "MANAGE_ROLES",
    required_bot_permissions = "MANAGE_ROLES"
)]
pub async fn role(_cx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

async fn is_user_allowed_to_operate_the_role(
    cx: &Context<'_>,
    user_roles: &Vec<RoleId>,
    target_role: RoleId,
) -> bool {
    let Ok(mut conn) = cx.data().database.get() else {
        return false;
    };
    let Ok(allowed_operators): Result<Vec<AllowedRole>, _> = allowed_roles::table
        .filter(allowed_roles::role_id.eq(TryInto::<i64>::try_into(target_role.get()).unwrap()))
        .select(AllowedRole::as_select())
        .load(&mut conn)
    else {
        return false;
    };

    for rule in allowed_operators {
        for role in user_roles {
            if rule.operator_role == TryInto::<i64>::try_into(role.get()).unwrap() {
                return true;
            }
        }
    }
    false
}

/// Add a role to user
#[poise::command(slash_command, ephemeral)]
pub async fn add(
    cx: Context<'_>,
    #[description = "User that gets the role"] user: Member,
    #[description = "Role to be given"] role: RoleId,
) -> Result<(), Error> {
    let member = cx.author_member().await.unwrap();
    let operator_roles = &member.roles;
    if !is_user_allowed_to_operate_the_role(&cx, operator_roles, role).await {
        cx.say("You are not allowed to operate this role.").await?;
        return Ok(());
    }
    if user.roles.contains(&role) {
        cx.say("User already has the role.").await?;
        return Ok(());
    }
    cx.http()
        .add_member_role(
            cx.guild_id().unwrap(),
            user.user.id,
            role,
            Some(format!("Given by @{} ({})", member.user.name, member.user.id).as_ref()),
        )
        .await?;
    cx.say(format!(
        "Gave <@{}> role <@&{}>.",
        user.user.id.get(),
        role.get()
    ))
    .await?;
    Ok(())
}

/// Remove a role from user
#[poise::command(slash_command, ephemeral)]
pub async fn remove(
    cx: Context<'_>,
    #[description = "User that gets the role"] user: Member,
    #[description = "Role to be given"] role: RoleId,
) -> Result<(), Error> {
    let member = cx.author_member().await.unwrap();
    let operator_roles = &member.roles;
    if !is_user_allowed_to_operate_the_role(&cx, &operator_roles, role).await {
        cx.say("You are not allowed to operate this role.").await?;
        return Ok(());
    }
    if !user.roles.contains(&role) {
        cx.say("User doesn't have the role.").await?;
        return Ok(());
    }
    cx.http()
        .remove_member_role(
            cx.guild_id().unwrap(),
            user.user.id,
            role,
            Some(format!("Removed by @{} ({})", member.user.name, member.user.id).as_ref()),
        )
        .await?;
    cx.say(format!(
        "Removed role <@&{}> from <@{}>.",
        role.get(),
        user.user.id.get()
    ))
    .await?;
    Ok(())
}

#[poise::command(
    slash_command,
    ephemeral,
    guild_only,
    rename = "temprole",
    subcommands("temp_add"),
    default_member_permissions = "MANAGE_ROLES",
    required_bot_permissions = "MANAGE_ROLES"
)]
pub async fn temp_role(_cx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add a temporary role to user
#[poise::command(slash_command, ephemeral, rename = "add")]
pub async fn temp_add(
    cx: Context<'_>,
    #[description = "User that gets the role"] user: Member,
    #[description = "Role to be given"] role: RoleId,
    #[description = "The duration that user will have the role"] mut duration: String,
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
    let member = cx.author_member().await.unwrap();
    let operator_roles = &member.roles;
    if !is_user_allowed_to_operate_the_role(&cx, operator_roles, role).await {
        cx.say("You are not allowed to operate this role.").await?;
        return Ok(());
    }
    if user.roles.contains(&role) {
        cx.say("User already has the role.").await?;
        return Ok(());
    }
    let queue = cx.data().queue.clone();
    let task = RemoveTempRole::new(cx.guild_id().unwrap(), user.user.id, role, duration_secs);
    queue.schedule_task(&task).await?;
    if duration.chars().last().map_or(false, |c| c.is_numeric()) {
        duration.push('s');
    }
    cx.http()
        .add_member_role(
            cx.guild_id().unwrap(),
            user.user.id,
            role,
            Some(
                format!(
                    "Given by @{} ({}) with a duration of {}",
                    member.user.name, member.user.id, duration
                )
                .as_ref(),
            ),
        )
        .await?;
    cx.say(format!(
        "Gave <@{}> role <@&{}> with a duration of **{}**.",
        user.user.id.get(),
        role.get(),
        duration
    ))
    .await?;
    Ok(())
}
