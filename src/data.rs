use std::sync::Arc;

use fang::AsyncQueue;
use serenity::{
    all::{Cache, CacheHttp, Http},
    prelude::TypeMapKey,
};

use crate::ConnectionPool;

#[derive(Debug)]
pub struct Data {
    pub(crate) database: ConnectionPool,
    pub(crate) queue: AsyncQueue,
}

pub struct ConnectionPoolKey;

impl TypeMapKey for ConnectionPoolKey {
    type Value = ConnectionPool;
}

pub struct QueueKey;

impl TypeMapKey for QueueKey {
    type Value = AsyncQueue;
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
