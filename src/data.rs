use std::sync::Arc;

use fang::AsyncQueue;
use serenity::{
    all::{Cache, CacheHttp, Http},
    prelude::TypeMapKey,
};

use crate::ConnectionPool;

pub struct Data {
    pub(crate) database: ConnectionPool,
    pub(crate) queue: AsyncQueue<fang::NoTls>,
}

pub struct ConnectionPoolKey;

impl TypeMapKey for ConnectionPoolKey {
    type Value = ConnectionPool;
}

pub struct QueueKey;

impl TypeMapKey for QueueKey {
    type Value = AsyncQueue<fang::NoTls>;
}

pub struct BotIdKey;

impl TypeMapKey for BotIdKey {
    type Value = u64;
}

#[derive(Clone)]
pub struct CacheHttpHolder(pub(crate) Arc<Cache>, pub(crate) Arc<Http>);

impl CacheHttp for CacheHttpHolder {
    fn http(&self) -> &Http {
        &self.1
    }

    fn cache(&self) -> Option<&Arc<Cache>> {
        Some(&self.0)
    }
}
