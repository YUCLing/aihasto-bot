use diesel::RunQueryDsl;
use fang::AsyncQueueable;
use serenity::all::{
    audit_log::Action, AuditLogEntry, Change, ChannelId, Context, GuildId, MemberAction, UserId,
};

use crate::{
    data::QueueKey,
    models::{
        guild_settings::GuildSettings,
        moderation_log::{CreateModerationLog, ModerationAction, ModerationLog},
    },
    util::{get_conn_from_serenity, send_moderation_logs_with_database_records},
};

use super::{moderation_dm::generate_dm_message, temp_role::RemoveTempRole};

pub async fn guild_audit_log_entry_create(cx: Context, entry: AuditLogEntry, guild_id: GuildId) {
    match entry.action {
        Action::Member(MemberAction::RoleUpdate) => {
            let changes = entry.changes.unwrap();
            for change in changes {
                if let Change::RolesRemove { old: _, new: roles } = change {
                    let removed_roles = roles.unwrap();
                    let user_id = UserId::new(entry.target_id.unwrap().get());
                    let queue = cx.data.read().await.get::<QueueKey>().unwrap().clone();
                    for role in removed_roles {
                        let task = RemoveTempRole::new(guild_id, user_id, role.id, 0);
                        if let Err(err) = queue.remove_task_by_metadata(&task).await {
                            log::warn!("Unable to remove temp role task: {}", err);
                        }
                    }
                }
            }
        }
        Action::Member(MemberAction::Update) => {
            let changes = entry.changes.unwrap();
            for change in changes {
                if let Change::CommunicationDisabledUntil {
                    old: _,
                    new: Some(_timestamp),
                } = change
                {
                    let cx = cx.clone();
                    let reason = entry.reason.clone();
                    tokio::spawn(async move {
                        let mut conn = get_conn_from_serenity(&cx)
                            .await
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
                                        logs.first().as_ref().unwrap(),
                                        &moderator,
                                        None::<ChannelId>,
                                    ),
                                )
                                .await;
                        }
                        if let Some(channel) =
                            GuildSettings::get(&mut conn, guild_id, "moderation_log_channel")
                        {
                            let channel = ChannelId::new(channel.parse().unwrap());
                            let _ = send_moderation_logs_with_database_records(
                                &mut conn, &cx, guild_id, channel, logs,
                            )
                            .await;
                        }
                    });
                }
            }
        }
        Action::Member(MemberAction::BanAdd) => {
            let mut conn = get_conn_from_serenity(&cx)
                .await
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
                send_moderation_logs_with_database_records(&mut conn, &cx, guild_id, channel, logs)
                    .await
                    .expect("Unable to send moderation logs.");
            }
        }
        _ => {}
    }
}
