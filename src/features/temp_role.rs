use fang::async_trait;
use fang::typetag;
use fang::AsyncQueueable;
use fang::AsyncRunnable;
use fang::FangError;
use fang::Scheduled;
use serde::{Deserialize, Serialize};
use serenity::all::GuildId;
use serenity::all::RoleId;
use serenity::all::UserId;

use crate::acquire_cache_http;

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub struct RemoveTempRole {
    guild_id: u64,
    user_id: u64,
    role_id: u64,
    #[serde(skip)]
    duration: u64,
}

impl RemoveTempRole {
    pub fn new<G: Into<GuildId>, U: Into<UserId>, R: Into<RoleId>>(
        guild: G,
        user: U,
        role: R,
        duration: u64,
    ) -> Self {
        RemoveTempRole {
            guild_id: guild.into().get(),
            user_id: user.into().get(),
            role_id: role.into().get(),
            duration,
        }
    }
}

#[typetag::serde]
#[async_trait]
impl AsyncRunnable for RemoveTempRole {
    async fn run(&self, _queue: &dyn AsyncQueueable) -> Result<(), FangError> {
        let http = acquire_cache_http();
        http.1
            .remove_member_role(
                GuildId::new(self.guild_id),
                UserId::new(self.user_id),
                RoleId::new(self.role_id),
                Some("Removed due to temporary role."),
            )
            .await
            .map_err(|x| FangError {
                description: x.to_string(),
            })?;
        Ok(())
    }

    fn uniq(&self) -> bool {
        true
    }

    fn cron(&self) -> Option<Scheduled> {
        Some(Scheduled::ScheduleOnce(
            chrono::Utc::now() + std::time::Duration::from_secs(self.duration),
        ))
    }

    fn max_retries(&self) -> i32 {
        3
    }
}
