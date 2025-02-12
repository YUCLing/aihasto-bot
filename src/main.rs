use std::{env, panic, process, sync::RwLock, time::Duration};

use commands::build_commands;
use data::{CacheHttpHolder, ConnectionPoolKey, Data, QueueKey};
use diesel::{r2d2::ConnectionManager, PgConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use event_handler::Handler;
use fang::{AsyncQueue, AsyncWorkerPool};
use lazy_static::lazy_static;
use logging::{log_framework_error, setup_logger, setup_panic_logger_hook};
use poise::{FrameworkError, PrefixFrameworkOptions};
use r2d2::{Pool, PooledConnection};
use serenity::{all::GatewayIntents, Client};
use tokio::signal;

const DATABASE_POOL_SIZE: u32 = 24;
const QUEUE_POOL_SIZE: u32 = 4;
const QUEUE_WORKERS: u32 = 4;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

type ConnManager = ConnectionManager<PgConnection>;
type ConnectionPool = Pool<ConnManager>;
#[allow(dead_code)]
type Connection = PooledConnection<ConnManager>;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

mod commands;
mod data;
mod embeds;
mod event_handler;
mod features;
mod logging;
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

async fn async_main() {
    let token = env::var("DISCORD_TOKEN").expect("Discord Bot token is required.");
    let db_url = env::var("DATABASE_URL").expect("Database URL is required.");

    let manager = ConnectionManager::<PgConnection>::new(db_url.clone());
    let pool = Pool::builder()
        .max_size(
            env::var("DATABASE_POOL_SIZE")
                .ok()
                .and_then(|x| x.parse().ok())
                .unwrap_or(DATABASE_POOL_SIZE),
        )
        .build(manager)
        .expect("Unable to create connection pool.");

    {
        let mut conn = pool.get().expect("Unable to get a database connection.");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Unable to migrate the database.");
    }

    log::info!("Database pool created.");

    let mut queue = AsyncQueue::builder()
        .uri(db_url)
        .max_pool_size(
            env::var("QUEUE_POOL_SIZE")
                .ok()
                .and_then(|x| x.parse().ok())
                .unwrap_or(QUEUE_POOL_SIZE),
        )
        .build();

    queue.connect().await.unwrap();

    log::info!("Queue created.");

    let options = poise::FrameworkOptions::<_, Error> {
        commands: build_commands(),
        prefix_options: PrefixFrameworkOptions {
            prefix: Some("!".to_string()),
            ..Default::default()
        },
        on_error: |err: FrameworkError<'_, Data, Error>| {
            Box::pin(async move {
                log_framework_error(&err);
            })
        },
        post_command: |cx: Context<'_>| {
            Box::pin(async move {
                log::info!(target: "aihasto_bot::command", "@{} ({}) executed \"{}\"", cx.author().name, cx.author().id, cx.command().qualified_name);
            })
        },
        ..Default::default()
    };

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
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES;

    let mut worker_pool: AsyncWorkerPool<AsyncQueue> = AsyncWorkerPool::builder()
        .number_of_workers(
            env::var("QUEUE_WORKERS")
                .ok()
                .and_then(|x| x.parse().ok())
                .unwrap_or(QUEUE_WORKERS),
        )
        .queue(queue.clone())
        .build();
    worker_pool.start().await;

    log::info!("Queue workers started.");

    log::info!("Starting bot...");

    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<ConnectionPoolKey>(pool)
        .type_map_insert::<QueueKey>(queue)
        .await
        .expect("Unable to create the client");

    {
        let mut cache_http = CACHE_HTTP.write().unwrap();
        *cache_http = Some(CacheHttpHolder(client.cache.clone(), client.http.clone()));
    }

    client.cache.set_max_messages(256);

    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        let shutdown = async move {
            log::info!("Shutting down...");
            tokio::select! {
                _ = async move {
                    shard_manager.shutdown_all().await;
                } => {},
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    log::error!("Unable to gracefully shutdown in time.");
                    process::exit(2);
                }
            }
            process::exit(0);
        };
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = signal::ctrl_c() => shutdown.await,
            _ = sigterm.recv() => shutdown.await
        };
    });

    if let Err(err) = client.start().await {
        log::error!("Client error: {err:?}");
    }

    process::exit(1);
}

fn main() {
    // behavior of logger can be configured with environment variables,
    // so loads .env before setting up the logger.
    if let Err(err) = dotenvy::dotenv() {
        if !err.not_found() {
            panic!("{err}");
        }
    }

    setup_logger().expect("Unable to setup logger.");
    setup_panic_logger_hook();

    let _guard;
    if let Ok(sentry_dsn) = env::var("SENTRY_DSN") {
        _guard = sentry::init((
            sentry_dsn,
            sentry::ClientOptions {
                release: Some(
                    format!(
                        "{}@{}{}",
                        env!("CARGO_PKG_NAME"),
                        env!("CARGO_PKG_VERSION"),
                        option_env!("BUILD_COMMIT")
                            .map(|x| format!("+{}", x))
                            .unwrap_or_default()
                    )
                    .into(),
                ),
                ..Default::default()
            },
        ));
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main());
}
