use diesel::RunQueryDsl;
use fang::AsyncQueueable;
use serenity::all::{
    audit_log::Action, AuditLogEntry, Change, ChannelId, Context, GuildId, MemberAction, UserId,
};

use crate::{
    data::{ConnectionPoolKey, QueueKey},
    models::{
        guild_settings::GuildSettings,
        moderation_log::{CreateModerationLog, ModerationAction, ModerationLog},
    },
    util::send_moderation_logs,
};

use super::{moderation_dm::generate_dm_message, temp_role::RemoveTempRole};

pub async fn guild_audit_log_entry_create(cx: Context, entry: AuditLogEntry, guild_id: GuildId) {
    match entry.action {
        Action::Member(MemberAction::RoleUpdate) => {
            let changes = entry.changes.unwrap();
            for change in changes {
                match change {
                    Change::RolesRemove { old: _, new: roles } => {
                        let removed_roles = roles.unwrap();
                        let user_id = UserId::new(entry.target_id.unwrap().get());
                        let queue = cx.data.read().await.get::<QueueKey>().unwrap().clone();
                        for role in removed_roles {
                            let task = RemoveTempRole::new(guild_id, user_id, role.id, 0);
                            match queue.remove_task_by_metadata(&task).await {
                                Err(err) => {
                                    println!("Unable to remove temp role task: {}", err);
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
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
                                let target = UserId::new(entry.target_id.unwrap().get());
                                if let Ok(moderator) = entry.user_id.to_user(&cx).await {
                                    let _ = target
                                        .dm(
                                            &cx,
                                            generate_dm_message(
                                                logs.get(0).as_ref().unwrap(),
                                                &moderator,
                                                None::<ChannelId>,
                                            ),
                                        )
                                        .await;
                                }
                                if let Some(channel) = GuildSettings::get(
                                    &mut conn,
                                    guild_id,
                                    "moderation_log_channel",
                                ) {
                                    let channel = ChannelId::new(channel.parse().unwrap());
                                    let _ = send_moderation_logs(&cx, channel, logs).await;
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
            if let Some(channel) = GuildSettings::get(&mut conn, guild_id, "moderation_log_channel")
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
