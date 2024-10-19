use crate::config;
use crate::controllers;
use crate::error::Result;
use crate::logger;
use crate::mailers::Mailer;
use crate::workers;
use crate::workers::WorkerOptions;
use axum::Router;
use migration::Migrator;
use migration::MigratorTrait;
use sea_orm::ConnectOptions;
use sea_orm::Database;
use sea_orm::DatabaseConnection;
use sea_orm::DbConn;
use std::sync::Arc;
use std::time::Duration;
use tokio::{net::TcpListener, signal};
use tower_http::trace::TraceLayer;
use tower_http::{compression::CompressionLayer, timeout::TimeoutLayer};

#[derive(Clone)]
pub struct Context {
    pub conn: DatabaseConnection,
}

async fn connect_db(config: &config::Database) -> Result<DbConn> {
    let mut opt = ConnectOptions::new(&config.uri);
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect_timeout(Duration::from_millis(config.connect_timeout))
        .idle_timeout(Duration::from_millis(config.idle_timeout))
        .sqlx_logging(config.enable_logging);

    if let Some(acquire_timeout) = config.acquire_timeout {
        opt.acquire_timeout(Duration::from_millis(acquire_timeout));
    }

    Ok(Database::connect(opt).await?)
}

pub async fn boot(config: &config::Config) -> Result<DatabaseConnection> {
    logger::init(&config.logger);
    let conn = connect_db(&config.database).await?;
    Migrator::up(&conn, None).await?;
    Ok(conn)
}

pub async fn boot_server_and_worker(
    config: &config::Config,
    conn: DatabaseConnection,
) -> Result<()> {
    let app = Router::new()
        .merge(controllers::defaults::route())
        .merge(controllers::files::routes())
        .merge(controllers::versions::routes())
        .layer((
            TraceLayer::new_for_http(),
            CompressionLayer::new(),
            // Graceful shutdown will wait for outstanding requests to complete. Add a timeout so
            // requests don't hang forever.
            TimeoutLayer::new(Duration::from_secs(10)),
        ))
        .with_state(Context { conn: conn.clone() });
    let mailer = Mailer::new(&config.mailer, &config.server.host)?;
    let worker_options = WorkerOptions {
        mailer: Arc::new(mailer),
        conn: conn.clone(),
        s3: Arc::new(config.s3.client()?),
        ak: config.ak.clone(),
    };
    let (check_handler, download_handler) = workers::start(worker_options)?;

    let listener = TcpListener::bind(config.server.full_url()).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    check_handler.abort();
    download_handler.abort();
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
