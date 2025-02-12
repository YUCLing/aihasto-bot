use fang::typetag;
use fang::AsyncQueueable;
use fang::AsyncRunnable;
use fang::FangError;
use fang::Scheduled;
use serde::{Deserialize, Serialize};
use serenity::async_trait;
use uuid::Uuid;

use crate::acquire_pool;

use super::case::delete_impl;

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub struct RemoveWarning {
    case_id: Uuid,
    #[serde(skip)]
    duration: u64,
}

impl RemoveWarning {
    pub fn new(case_id: Uuid, duration: u64) -> Self {
        RemoveWarning { case_id, duration }
    }
}

#[typetag::serde]
#[async_trait]
impl AsyncRunnable for RemoveWarning {
    async fn run(&self, _queue: &dyn AsyncQueueable) -> Result<(), FangError> {
        delete_impl(&acquire_pool(), self.case_id.to_string())
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
