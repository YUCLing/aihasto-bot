use diesel::{
    query_dsl::methods::{FilterDsl, FindDsl, LimitDsl, SelectDsl},
    ExpressionMethods, OptionalExtension, RunQueryDsl, SelectableHelper,
};
use poise::CreateReply;
use serenity::all::{
    ChannelId, Colour, CreateActionRow, CreateEmbed, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption, EditChannel, ReactionType,
};

use crate::{models::voice_channel::VoiceChannel, schema::voice_channels, Context, Error};

async fn own_voice_channel_check(cx: Context<'_>) -> Result<bool, Error> {
    let Some(channel) = cx.guild_channel().await else {
        return Ok(false);
    };
    let mut conn = cx.data().database.get()?;
    let results = voice_channels::table
        .filter(voice_channels::id.eq(TryInto::<i64>::try_into(channel.id.get()).unwrap()))
        .filter(voice_channels::creator.eq(TryInto::<i64>::try_into(cx.author().id.get()).unwrap()))
        .limit(1)
        .select(VoiceChannel::as_select())
        .load(&mut conn)?;
    if results.is_empty() {
        cx.send(CreateReply {
            ephemeral: Some(true),
            content: Some("You don't own this channel".to_string()),
            ..Default::default()
        })
        .await?;
    }
    Ok(!results.is_empty())
}

#[poise::command(
    slash_command,
    ephemeral,
    guild_only,
    rename = "tempvoice",
    subcommands("rename", "limit", "delete", "kick"),
    required_bot_permissions = "MANAGE_CHANNELS|MOVE_MEMBERS"
)]
pub async fn temp_voice(_cx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Rename your voice channel.
#[poise::command(slash_command, guild_only, check = "own_voice_channel_check")]
pub async fn rename(
    cx: Context<'_>,
    #[description = "New name of the voice channel"]
    #[rest]
    name: String,
) -> Result<(), Error> {
    let mut channel = cx.guild_channel().await.unwrap();
    let actor = cx.author();
    let reason = format!("Renamed by @{} ({})", actor.name, actor.id);
    match channel
        .edit(
            &cx,
            EditChannel::new().name(&name).audit_log_reason(&reason),
        )
        .await
    {
        Ok(_) => {
            cx.say(format!("The channel has been renamed to {}", name))
                .await
        }
        Err(_) => cx.say("Failed to rename the channel.").await,
    }?;
    Ok(())
}

/// Set your voice channel's max user count.
#[poise::command(slash_command, guild_only, check = "own_voice_channel_check")]
pub async fn limit(
    cx: Context<'_>,
    #[description = "Max user count of the channel, ignore to remove the limit."]
    #[max = 99]
    #[min = 1]
    count: Option<u32>,
) -> Result<(), Error> {
    let mut channel = cx.guild_channel().await.unwrap();
    match channel
        .edit(&cx, EditChannel::new().user_limit(count.unwrap_or(0)))
        .await
    {
        Ok(_) => {
            cx.say(format!(
                "The user limit of the channel has been set to {}",
                count
                    .map(|x| x.to_string())
                    .unwrap_or("unlimited".to_string())
            ))
            .await
        }
        Err(_) => cx.say("Failed to rename the channel.").await,
    }?;
    Ok(())
}

/// Delete your voice channel.
#[poise::command(
    slash_command,
    ephemeral,
    guild_only,
    check = "own_voice_channel_check"
)]
pub async fn delete(cx: Context<'_>) -> Result<(), Error> {
    let channel = cx.guild_channel().await.unwrap();
    channel.delete(&cx).await?;
    Ok(())
}

/// Kick someone from your voice channel.
#[poise::command(
    slash_command,
    ephemeral,
    guild_only,
    check = "own_voice_channel_check"
)]
pub async fn kick(cx: Context<'_>) -> Result<(), Error> {
    let channel = cx.guild_channel().await.unwrap();
    let options: Vec<CreateSelectMenuOption> = channel
        .members(cx)?
        .iter()
        .filter(|x| x.user.id != cx.author().id)
        .map(|x| {
            CreateSelectMenuOption::new(
                x.nick.clone().unwrap_or(x.user.display_name().to_string()),
                x.user.id.get().to_string(),
            )
            .description(x.user.name.clone())
            .emoji(ReactionType::Unicode("👤".to_string()))
        })
        .collect();
    if options.is_empty() {
        cx.say("There's no one else to be kicked.").await?;
        return Ok(());
    }
    let select_menu =
        CreateSelectMenu::new("voice_kick_user", CreateSelectMenuKind::String { options })
            .placeholder("The user that will be kicked.");
    let reply = CreateReply {
        ephemeral: Some(true),
        embeds: vec![CreateEmbed::new()
            .color(Colour::ORANGE)
            .title("Kick a User")
            .description(
                "To kick a user from your voice channel, select him/her in the following menu.",
            )],
        components: Some(vec![CreateActionRow::SelectMenu(select_menu)]),
        ..Default::default()
    };
    cx.send(reply).await?;

    Ok(())
}

/// Force delete a temporary voice channel.
#[poise::command(
    slash_command,
    ephemeral,
    guild_only,
    rename = "delete_tempvoice_channel",
    required_bot_permissions = "MANAGE_CHANNELS",
    default_member_permissions = "MANAGE_CHANNELS"
)]
pub async fn admin_delete(
    cx: Context<'_>,
    #[description = "The channel to be deleted"]
    #[channel_types("Voice")]
    channel: ChannelId,
) -> Result<(), Error> {
    let mut conn = cx.data().database.get()?;
    let id: Option<i64> = voice_channels::table
        .find(TryInto::<i64>::try_into(channel.get()).unwrap())
        .select(voice_channels::id)
        .get_result(&mut conn)
        .optional()?;
    if id.is_none() {
        cx.say("This voice channel is not managed by temp voice.")
            .await?;
        return Ok(());
    }
    channel.delete(&cx).await?;
    cx.say("Channel has been deleted.").await?;
    Ok(())
}
