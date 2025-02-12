use poise::Command;

use crate::{data::Data, Error};

mod beep;
mod case;
mod manage;
mod moderation;
mod role;
mod temp_voice;

pub fn build_commands() -> Vec<Command<Data, Error>> {
    vec![
        beep::beep(),
        case::case(),
        manage::sman(),
        moderation::slowmode(),
        moderation::inspect(),
        moderation::context_menu_inspect(),
        moderation::warning(),
        moderation::warning_with_interaction(),
        moderation::flood(),
        moderation::flood_with_interaction(),
        moderation::unflood(),
        moderation::softban(),
        moderation::unsoftban(),
        role::role(),
        role::temp_role(),
        temp_voice::temp_voice(),
        temp_voice::admin_delete(),
    ]
}
