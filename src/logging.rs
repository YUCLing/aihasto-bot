use std::{io, panic};

use fern::colors::{Color, ColoredLevelConfig};
use poise::FrameworkError;

use crate::{data::Data, Error};

pub fn setup_logger() -> Result<(), fern::InitError> {
    let colors = ColoredLevelConfig::new().info(Color::BrightBlue);
    fern::Dispatch::new()
        .format(move |out, message, record| {
            let time = chrono::Local::now();
            out.finish(format_args!(
                "[{} {} {}] {}",
                time.format("%Y-%m-%d %H:%M:%S%.3f"),
                record.target(),
                colors.color(record.level()),
                message
            ));
        })
        .chain(
            fern::Dispatch::new()
                .level(log::LevelFilter::Warn)
                .level_for("aihasto_bot", log::LevelFilter::Info)
                .chain(io::stdout()),
        )
        .apply()?;
    Ok(())
}

pub fn setup_panic_logger_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let error_with_message = |str: &String| {
            log::error!(
                "Panic occurred at {}: {}",
                info.location()
                    .and_then(|x| Some(x.to_string()))
                    .unwrap_or("unknown".to_string()),
                str
            );
        };
        let payload = info.payload();
        if let Some(msg) = payload.downcast_ref::<&str>() {
            error_with_message(&msg.to_string());
        } else if let Some(msg) = payload.downcast_ref::<String>() {
            error_with_message(msg);
        } else {
            log::error!(
                "Panic occurred at {}",
                info.location()
                    .and_then(|x| Some(x.to_string()))
                    .unwrap_or("unknown".to_string())
            );
        }
        // still calls the default hook for detailed information.
        default_hook(info);
    }));
}

pub fn log_framework_error(err: &FrameworkError<'_, Data, Error>) {
    match err {
        FrameworkError::Command { error, ctx: _, .. } => {
            log::warn!("Error when processing command: {}", error.to_string());
        }
        FrameworkError::CommandPanic {
            payload, ctx: _, ..
        } => {
            log::warn!(
                "Panic when processing command: {}",
                payload.clone().unwrap_or("no error message".to_string())
            );
        }
        FrameworkError::UnknownCommand { .. }
        | FrameworkError::DmOnly { .. }
        | FrameworkError::CooldownHit { .. }
        | FrameworkError::NotAnOwner { .. }
        | FrameworkError::MissingBotPermissions { .. }
        | FrameworkError::MissingUserPermissions { .. }
        | FrameworkError::SubcommandRequired { .. }
        | FrameworkError::GuildOnly { .. }
        | FrameworkError::NsfwOnly { .. }
        | FrameworkError::CommandStructureMismatch { .. }
        | FrameworkError::UnknownInteraction { .. }
        | FrameworkError::CommandCheckFailed { .. } => {}
        _ => {
            log::warn!("Encountered unhandled error: {err:?}");
        }
    }
}
