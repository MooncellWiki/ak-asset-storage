# Configuration Documentation

This document describes the configuration options for the application. The configuration is stored in TOML format.

## Configuration Structure

The configuration is divided into several main sections:

- Logger
- Server
- Database
- Mailer
- AK (Arknights)
- S3 Storage
- Sentry

## Logger Configuration

Controls the application's logging behavior.

| Field             | Description                     | Options                                   |
| ----------------- | ------------------------------- | ----------------------------------------- |
| `enable`          | Enable log writing to stdout    | `true`/`false`                            |
| `level`           | Set logging level               | `trace`, `debug`, `info`, `warn`, `error` |
| `format`          | Set logger format               | `compact`, `pretty`, `json`               |
| `override_filter` | Override default tracing filter | Any valid tracing filter string           |

## Server Configuration

Configures the web server settings.

```toml
[server]
port = 25150
host = "http://localhost"
```

| Field     | Description                                      |
| --------- | ------------------------------------------------ |
| `binding` | Server binding address (defaults to "localhost") |
| `port`    | Port number for the server                       |
| `host`    | Web server host URL                              |

## Database Configuration

Database connection and pool settings.

```toml
[database]
uri = "postgres://user:password@localhost:5432/dbname"
```

| Field | Description             |
| ----- | ----------------------- |
| `uri` | Database connection URI |

## Mailer Configuration

Email sending configuration using SMTP.

```toml
[mailer.smtp]
host = "smtp.example.com"
port = 465

auth.user = "user@example.com"
auth.password = "password"
```

| Field           | Description                  |
| --------------- | ---------------------------- |
| `host`          | SMTP server host             |
| `port`          | SMTP server port             |
| `auth.user`     | SMTP authentication username |
| `auth.password` | SMTP authentication password |

## AK Configuration

Arknights-specific configuration.

```toml
[ak]
asset_url = "https://ak.hycdn.cn/assetbundle/official/Android/assets"
conf_url = "https://ak-conf.hypergryph.com/config/prod/official/Android"
```

| Field       | Description                 |
| ----------- | --------------------------- |
| `asset_url` | URL for asset bundles       |
| `conf_url`  | URL for configuration files |

## S3 Storage Configuration

Amazon S3 compatible storage configuration.

```toml
[s3]
endpoint = "http://127.0.0.1:29000"
bucket_name = "bucket-name"
access_key_id = "access-key"
secret_access_key = "secret-key"
with_virtual_hosted_style_request = false
```

| Field                               | Description                          |
| ----------------------------------- | ------------------------------------ |
| `endpoint`                          | S3 endpoint URL                      |
| `bucket_name`                       | S3 bucket name                       |
| `access_key_id`                     | S3 access key ID                     |
| `secret_access_key`                 | S3 secret access key                 |
| `with_virtual_hosted_style_request` | Enable virtual hosted style requests |

## Sentry

```toml
[sentry]
dsn = "dsn"
traces_sample_rate = 0.1
```

| Field                | Description                  |
| -------------------- | ---------------------------- |
| `dsn`                | sentry dsn                   |
| `traces_sample_rate` | sample rate for transactions |

## Example Configuration

See the `example.toml` file for a complete example configuration.
