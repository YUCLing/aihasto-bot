use std::{env, sync::RwLock};

use commands::build_commands;
use data::{CacheHttpHolder, ConnectionPoolKey, Data, QueueKey};
use diesel::{r2d2::ConnectionManager, PgConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use event_handler::Handler;
use fang::{AsyncQueue, AsyncWorkerPool};
use lazy_static::lazy_static;
use poise::PrefixFrameworkOptions;
use r2d2::{Pool, PooledConnection};
use serenity::{all::GatewayIntents, Client};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

type ConnManager = ConnectionManager<PgConnection>;
type ConnectionPool = Pool<ConnManager>;
type Connection = PooledConnection<ConnManager>;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

mod commands;
mod data;
mod embeds;
mod event_handler;
mod features;
mod models;
mod schema;
mod util;

lazy_static! {
    static ref CACHE_HTTP: RwLock<Option<CacheHttpHolder>> = RwLock::new(None);
}

fn acquire_cache_http() -> CacheHttpHolder {
    CACHE_HTTP
        .read()
        .unwrap()
        .as_ref()
        .expect("Why the hell we can't get the holder?")
        .clone()
}

#[tokio::main]
async fn main() {
    if let Err(err) = dotenvy::dotenv() {
        if !err.not_found() {
            panic!("{err}");
        }
    }

    let token = env::var("DISCORD_TOKEN").expect("Discord Bot token is required.");
    let db_url = env::var("DATABASE_URL").expect("Database URL is required.");

    let options = poise::FrameworkOptions::<_, Error> {
        commands: build_commands(),
        prefix_options: PrefixFrameworkOptions {
            prefix: Some("!".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let manager = ConnectionManager::<PgConnection>::new(db_url.clone());
    let pool = Pool::builder()
        .build(manager)
        .expect("Unable to create connection pool.");

    {
        let mut conn = pool.get().expect("Unable to get a database connection.");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Unable to migrate the database.");
    }

    println!("Database connection pool created.");

    let mut queue = AsyncQueue::builder()
        .uri(db_url)
        .max_pool_size(4u32)
        .build();

    queue.connect().await.unwrap();

    println!("Queue created.");

    let pool_clone = pool.clone();
    let queue_clone = queue.clone();
    let framework = poise::Framework::builder()
        .setup(move |cx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(cx, &framework.options().commands)
                    .await
                    .expect("Unable to register commands");
                Ok(Data {
                    database: pool_clone,
                    queue: queue_clone,
                })
            })
        })
        .options(options)
        .build();

    let intents = GatewayIntents::privileged()
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MODERATION
        | GatewayIntents::GUILD_MESSAGES;

    let queue_clone = queue.clone();
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<ConnectionPoolKey>(pool)
        .type_map_insert::<QueueKey>(queue_clone)
        .await
        .expect("Unable to create the client");

    {
        let mut cache_http = CACHE_HTTP.write().unwrap();
        *cache_http = Some(CacheHttpHolder(client.cache.clone(), client.http.clone()));
    }

    client.cache.set_max_messages(256);

    println!("Starting queue workers...");
    let mut pool: AsyncWorkerPool<AsyncQueue> = AsyncWorkerPool::builder()
        .number_of_workers(4u32)
        .queue(queue)
        .build();
    pool.start().await;

    println!("Starting bot...");

    if let Err(err) = client.start().await {
        eprintln!("Client error: {err:?}");
    }
}
