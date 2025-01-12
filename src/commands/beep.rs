use poise::CreateReply;
use serenity::all::CreateEmbed;

use crate::{Context, Error};

#[poise::command(prefix_command, aliases("info", "version"))]
pub async fn beep(cx: Context<'_>) -> Result<(), Error> {
    let current_user = cx.http().get_current_user().await?;
    cx.send(CreateReply {
        embeds: vec![CreateEmbed::new()
            .color(0x330064)
            .description(format!(
                "# {}",
                current_user
                    .global_name
                    .clone()
                    .unwrap_or(current_user.name.clone())
            ))
            .fields([
                ("Version", format!("`{}`", env!("CARGO_PKG_VERSION")), true),
                (
                    "Build",
                    format!(
                        "`{}`",
                        &option_env!("BUILD_COMMIT").unwrap_or("unknown")[0..7]
                    ),
                    true,
                ),
                ("Built at", format!("<t:{}>", env!("BUILD_TIME")), false),
            ])
            .thumbnail(
                current_user
                    .avatar_url()
                    .unwrap_or(current_user.default_avatar_url()),
            )],
        ..Default::default()
    })
    .await?;
    Ok(())
}
