use poise::Command;

use crate::{Data, Error};

mod manage;
mod moderation;
mod role;
mod temp_voice;

pub fn build_commands() -> Vec<Command<Data, Error>> {
    vec![
        manage::sman(),

        moderation::slowmode(),
        role::role(),
        role::temp_role(),

        temp_voice::temp_voice()
    ]
}