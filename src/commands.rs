use poise::Command;

use crate::{data::Data, Error};

mod beep;
mod manage;
mod moderation;
mod role;
mod temp_voice;

pub fn build_commands() -> Vec<Command<Data, Error>> {
    vec![
        beep::beep(),
        manage::sman(),
        moderation::slowmode(),
        moderation::inspect(),
        moderation::warning(),
        moderation::flood(),
        moderation::reason(),
        role::role(),
        role::temp_role(),
        temp_voice::temp_voice(),
        temp_voice::admin_delete(),
    ]
}
