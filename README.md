# Configuration Documentation

This document describes the configuration options for the application. The configuration is stored in TOML format.

## Configuration Structure

The configuration is divided into several main sections:

- Logger
- Server
- Database
- Mailer (SMTP)
- AK (Arknights)
- S3 Storage
- Sentry (Optional)
- Torappu (Optional)

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

| Field                        | Description                   | Default |
| ---------------------------- | ----------------------------- | ------- |
| `uri`                        | Database connection URI       | -       |
| `max_connections`            | Maximum database connections  | `None`  |
| `connection_timeout_seconds` | Connection timeout in seconds | `None`  |

## Mailer Configuration

Email sending configuration using SMTP.

```toml
[mailer.smtp]
host = "smtp.example.com"
port = 465

auth.user = "user@example.com"
auth.password = "password"
```

| Field           | Description                     |
| --------------- | ------------------------------- |
| `host`          | SMTP server host                |
| `port`          | SMTP server port                |
| `from_email`    | Email address to send from      |
| `to_email`      | Email address to send to        |
| `frontend_url`  | URL of the frontend application |
| `auth.user`     | SMTP authentication username    |
| `auth.password` | SMTP authentication password    |

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

## Sentry Configuration (Optional)

Optional configuration for Sentry error tracking and monitoring.

```toml
[sentry]
dsn = "https://your-sentry-dsn@sentry.io/project-id"
traces_sample_rate = 1.0
```

| Field                | Description                                    | Required |
| -------------------- | ---------------------------------------------- | -------- |
| `dsn`                | Sentry DSN for error reporting                 | No       |
| `traces_sample_rate` | Sampling rate for performance traces (0.0-1.0) | No       |

## Torappu Configuration (Optional)

Optional configuration for Torappu service integration.

```toml
[torappu]
token = "your-torappu-token-here"
asset_base_path = "/assets"

[torappu.docker]
image_url = "your-docker-image:latest"
container_name = "ak-asset-container"
env_vars = [ "TZ=Asia/Shanghai" ]
volume_mapping = "./data:/app/data"
docker_host = "/var/run/docker.sock"

[torappu.github]
owner = "your-username"
repo = "your-repo"
workflow_id = "workflow-file.yml"
ref = "main"
token = "github-token"
```

| Field                   | Description                  | Required |
| ----------------------- | ---------------------------- | -------- |
| `token`                 | Torappu authentication token | No       |
| `asset_base_path`       | Base path for assets         | No       |
| `docker.image_url`      | Docker image to deploy       | No       |
| `docker.container_name` | Container name               | No       |
| `docker.env_vars`       | Environment variables        | No       |
| `docker.volume_mapping` | Volume mappings              | No       |
| `docker.docker_host`    | Docker daemon host           | No       |
| `github.owner`          | GitHub repository owner      | No       |
| `github.repo`           | GitHub repository name       | No       |
| `github.workflow_id`    | GitHub Actions workflow ID   | No       |
| `github.ref`            | Git branch or tag            | No       |
| `github.token`          | GitHub personal access token | No       |

## Example Configuration

See the `example.toml` file for a complete example configuration.
