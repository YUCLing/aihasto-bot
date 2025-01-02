use poise::Command;

use crate::{data::Data, Error};

mod manage;
mod moderation;
mod role;
mod temp_voice;

pub fn build_commands() -> Vec<Command<Data, Error>> {
    vec![
        manage::sman(),
        moderation::slowmode(),
        moderation::inspect(),
        moderation::warning(),
        role::role(),
        role::temp_role(),
        temp_voice::temp_voice(),
    ]
}
