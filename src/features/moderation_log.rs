use diesel::RunQueryDsl;
use serenity::all::{
    audit_log::Action, AuditLogEntry, Change, ChannelId, Context, GuildId, MemberAction, UserId,
};

use crate::{
    data::ConnectionPoolKey,
    models::{
        guild_settings::GuildSettings,
        moderation_log::{CreateModerationLog, ModerationAction, ModerationLog},
    },
    util::send_moderation_logs,
};

pub async fn guild_audit_log_entry_create(cx: Context, entry: AuditLogEntry, guild_id: GuildId) {
    match entry.action {
        Action::Member(MemberAction::Update) => {
            let changes = entry.changes.unwrap();
            for change in changes {
                match change {
                    Change::CommunicationDisabledUntil { old: _, new } => {
                        if let Some(_timestamp) = new {
                            let cx = cx.clone();
                            let reason = entry.reason.clone();
                            tokio::spawn(async move {
                                let mut conn = {
                                    cx.data
                                        .read()
                                        .await
                                        .get::<ConnectionPoolKey>()
                                        .unwrap()
                                        .get()
                                }
                                .expect("Unable to get database connection.");
                                let logs = ModerationLog::insert()
                                    .values([CreateModerationLog::new(
                                        guild_id,
                                        ModerationAction::Timeout,
                                        entry.target_id.unwrap().get(),
                                        Some(entry.user_id),
                                        reason,
                                    )])
                                    .get_results(&mut conn)
                                    .expect("Unable to log timeout.");
                                if let Some(channel) = GuildSettings::get(
                                    &mut conn,
                                    guild_id,
                                    "set_moderation_log_channel",
                                ) {
                                    let channel = ChannelId::new(channel.parse().unwrap());
                                    send_moderation_logs(&cx, channel, logs)
                                        .await
                                        .expect("Unable to send moderation logs.");
                                }
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        Action::Member(MemberAction::BanAdd) => {
            let mut conn = {
                cx.data
                    .read()
                    .await
                    .get::<ConnectionPoolKey>()
                    .unwrap()
                    .get()
            }
            .expect("Unable to get database connection.");
            let logs = ModerationLog::insert()
                .values([CreateModerationLog::new(
                    guild_id,
                    ModerationAction::Timeout,
                    UserId::new(entry.target_id.unwrap().get()),
                    Some(entry.user_id),
                    entry.reason,
                )])
                .get_results(&mut conn)
                .expect("Unable to log ban.");
            if let Some(channel) =
                GuildSettings::get(&mut conn, guild_id, "set_moderation_log_channel")
            {
                let channel = ChannelId::new(channel.parse().unwrap());
                send_moderation_logs(&cx, channel, logs)
                    .await
                    .expect("Unable to send moderation logs.");
            }
        }
        _ => {}
    }
}
