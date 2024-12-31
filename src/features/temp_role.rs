use chrono::DateTime;
use fang::AsyncQueueable;
use fang::AsyncRunnable;
use fang::async_trait;
use fang::FangError;
use fang::typetag;
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
    remove_time: i64
}

impl RemoveTempRole {
    pub fn new<G: Into<GuildId>, U: Into<UserId>, R: Into<RoleId>>(guild: G, user: U, role: R, duration: u64) -> Self {
        RemoveTempRole {
            guild_id: guild.into().get(),
            user_id: user.into().get(),
            role_id: role.into().get(),
            remove_time: (chrono::Utc::now() + std::time::Duration::from_secs(duration)).timestamp()
        }
    }
}

#[typetag::serde]
#[async_trait]
impl AsyncRunnable for RemoveTempRole {
    async fn run(&self, _queue: &mut dyn AsyncQueueable) -> Result<(), FangError> {
        let http = acquire_cache_http();
        let guild_id = GuildId::new(self.guild_id);
        let member = guild_id.member(&http, UserId::new(self.user_id)).await.map_err(|x| FangError { description: x.to_string() })?;
        member.remove_role(&http.1, RoleId::new(self.role_id)).await.map_err(|x| FangError { description: x.to_string() })?;
        Ok(())
    }

    fn uniq(&self) -> bool {
        true
    }

    fn cron(&self) -> Option<Scheduled> {
        Some(Scheduled::ScheduleOnce(DateTime::from_timestamp(self.remove_time, 0).unwrap()))
    }

    fn max_retries(&self) -> i32 {
        3
    }
}