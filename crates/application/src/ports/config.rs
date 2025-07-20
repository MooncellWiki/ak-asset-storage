use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_variant::to_variant_name;

/// Configuration provider interface for application layer
#[async_trait]
pub trait ConfigProvider: Send + Sync {
    /// Get database connection string
    fn database_config(&self) -> &DatabaseConfig;

    /// Get server configuration
    fn server_config(&self) -> &ServerConfig;

    /// Get external service configurations
    fn ak_api_config(&self) -> &AkApiConfig;
    fn s3_config(&self) -> &S3Config;
    fn smtp_config(&self) -> &Option<SmtpConfig>;

    /// Get logging configuration
    fn logger_config(&self) -> &LoggerConfig;

    /// Get sentry configuration if enabled
    fn sentry_config(&self) -> &SentryConfig;

    /// Get torappu configuration
    fn torappu_config(&self) -> &TorappuConfig;
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// The URI for connecting to the database. For example:
    /// * Postgres: `postgres://root:12341234@localhost:5432/myapp_development`
    /// * Sqlite: `sqlite://db.sqlite?mode=rwc`
    pub uri: String,
    pub max_connections: Option<u32>,
    pub connection_timeout_seconds: Option<u64>,
}

/// Server configuration for application use
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// The address on which the server should listen on for incoming
    /// connections.
    #[serde(default = "default_binding")]
    pub binding: String,
    /// The port on which the server should listen for incoming connections.
    pub port: i32,
    /// The webserver host
    pub host: String,
}

fn default_binding() -> String {
    "localhost".to_string()
}

impl ServerConfig {
    #[must_use]
    pub fn full_url(&self) -> String {
        format!("{}:{}", self.binding, self.port)
    }
}

/// AK API configuration for application use
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AkApiConfig {
    pub conf_url: String,
    pub asset_url: String,
}

/// S3 configuration for application use
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct S3Config {
    pub endpoint: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub bucket_name: String,
    pub with_virtual_hosted_style_request: bool,
}

/// SMTP configuration for application use
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SmtpConfig {
    /// SMTP host. for example: localhost, smtp.gmail.com etc.
    pub host: String,
    /// SMTP port/
    pub port: u16,
    /// Auth SMTP server
    pub auth: MailerAuthConfig,
    pub from_email: String,
    pub to_email: String,
    pub frontend_url: String,
}

/// Authentication details for the mailer
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MailerAuthConfig {
    /// User
    pub user: String,
    /// Password
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}
impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        to_variant_name(self).expect("only enum supported").fmt(f)
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum LogFormat {
    #[default]
    Compact,
    Pretty,
    Json,
}

/// Logger configuration for application use
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LoggerConfig {
    /// Enable log write to stdout
    pub enable: bool,

    /// Set the logger level.
    ///
    /// * options: `trace` | `debug` | `info` | `warn` | `error`
    pub level: LogLevel,

    /// Set the logger format.
    ///
    /// * options: `compact` | `pretty` | `json`
    pub format: LogFormat,

    /// Override our custom tracing filter.
    ///
    /// Set this to your own filter if you want to see traces from internal
    /// libraries. See more [here](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives)
    pub override_filter: Option<String>,
}

/// Sentry configuration for application use
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SentryConfig {
    pub dsn: String,
    pub traces_sample_rate: f32,
}

/// Torappu configuration for application use
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TorappuConfig {
    pub token: String,
}
