use serenity::all::{CacheHttp, ChannelId, CreateMessage};

use crate::{models::moderation_log::ModerationLog, Error};

pub fn parse_duration_to_seconds<T: AsRef<str>>(duration: T) -> Result<u64, String> {
    let mut total_seconds = 0;
    let mut current_number = String::new();

    // Iterate through each character of the input string
    for ch in duration.as_ref().chars() {
        match ch {
            '0'..='9' => {
                // Collect digits for the current number
                current_number.push(ch);
            }
            's' | 'm' | 'h' | 'd' | 'w' => {
                // If we encounter a time unit (s, m, or h), process the current number
                if let Ok(value) = current_number.parse::<u64>() {
                    match ch {
                        's' => total_seconds += value,
                        'm' => total_seconds += value * 60,
                        'h' => total_seconds += value * 3600,
                        'd' => total_seconds += value * 86400,
                        'w' => total_seconds += value * 604800,
                        _ => return Err(format!("Unsupported unit: {}", ch)),
                    }
                    current_number.clear(); // Reset the current number for the next part
                } else {
                    return Err("Invalid number".to_string());
                }
            }
            _ => return Err("Invalid character in duration string".to_string()), // Handle invalid characters
        }
    }

    // After finishing the loop, if there's any leftover number, assume it's in seconds
    if !current_number.is_empty() {
        if let Ok(value) = current_number.parse::<u64>() {
            total_seconds += value; // Treat as seconds if no unit was given
        } else {
            return Err("Invalid number".to_string());
        }
    }

    Ok(total_seconds)
}

pub async fn send_moderation_logs<
    T: CacheHttp,
    U: Into<ChannelId>,
    V: IntoIterator<Item = ModerationLog>,
>(
    cx: &T,
    channel: U,
    logs: V,
) -> Result<(), Error> {
    let channel = channel.into();
    for log in logs {
        channel
            .send_message(cx, CreateMessage::new().embed(log.clone().into()))
            .await?;
    }
    Ok(())
}
