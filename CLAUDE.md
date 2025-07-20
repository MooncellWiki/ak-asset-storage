# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Arknights Asset Storage** - A Rust-based service for storing and managing Arknights game assets with a Vue.js frontend. Uses clean architecture with separate crates for different layers.

## Architecture

The project follows clean architecture with these layers:

- **Application Layer** (`crates/application/`): Core business logic, entities, and use cases
- **Infrastructure Layer** (`crates/infrastructure/`): External services (database, S3, email, HTTP clients)
- **Web Layer** (`crates/web/`): HTTP API endpoints and web interface
- **CLI Layer** (`crates/cli/`): Command-line interface for background tasks

### Clean Architecture Layers

**Entities** (`crates/application/src/entities/`):

- `Bundle`: Collection of game assets with metadata
- `File`: Individual asset files with version tracking
- `Version`: Game version information and asset mapping

**Use Cases** (`crates/application/src/services/`):

- `AssetDownloadService`: Downloads and validates game assets
- `VersionCheckService`: Monitors for new game versions
- `SyncTask`: Orchestrates asset synchronization workflows

**Ports** (`crates/application/src/ports/`):

- Repository interfaces for data persistence
- External service abstractions for S3, email, HTTP clients
- Configuration provider interface

**Adapters** (`crates/infrastructure/src/`):

- PostgreSQL repositories (`persistence/postgres/`)
- S3-compatible storage client (`external/s3_storage_client.rs`)
- Arknights API client (`external/ak_api_client.rs`)
- SMTP email client (`external/smtp_client.rs`)

## Development Commands

### Setup

```bash
# Install dependencies
just init
pnpm install

# Start infrastructure services
docker-compose up -d

# Run database migrations
sqlx migrate run
```

### Backend (Rust)

```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run specific test
cargo test -p ak-asset-storage-application --test test_complete_workflow

# Run web server
cargo run --bin ak-asset-storage server

# Run background worker
cargo run --bin ak-asset-storage worker

# Seed database with CSV data
cargo run --bin ak-asset-storage seed --csv-path data.csv

# Check specific crate
cargo check -p ak-asset-storage-web
```

### Frontend (Vue.js)

```bash
# Install dependencies
pnpm install

# Development server
pnpm dev

# Build for production
pnpm build

# Type checking
pnpm typecheck

# Linting
pnpm lint
pnpm lint:fix

# Generate API types from OpenAPI schema
pnpm api

# Preview production build
pnpm preview
```

## Configuration

Create `config.toml` based on `example.toml`:

- **Database**: PostgreSQL connection URI
- **S3 Storage**: MinIO/S3 compatible storage settings
- **Arknights**: Asset and configuration URLs for game data
- **SMTP**: Email configuration for notifications
- **Server**: Web server binding and port settings
- **Torappu**: Torappu integration token for enhanced features

### Core Entities

- **Bundle**: Collection of game assets
- **Version**: Game version information
- **File**: Individual asset files with metadata

### Services

- **AssetDownloadService**: Downloads assets from Arknights servers
- **VersionCheckService**: Monitors for new game versions
- **SyncTask**: Background synchronization of assets

### External Integrations

- **PostgreSQL**: Primary database via SQLx
- **MinIO/S3**: Asset storage backend
- **Arknights API**: Official game asset endpoints
- **SMTP**: Email notifications for sync completion

## Testing

- **Integration tests**: In application crate under `tests/integration/`

## Database

- **Migrations**: In `migrations/` directory
- **Schema**: See `docs/sql/v\d.md`
- **Connection**: Managed through SQLx with PostgreSQL

## Frontend Structure

- **Framework**: Vue 3 with TypeScript
- **UI Library**: Naive UI
- **Styling**: UnoCSS
- **Router**: Vue Router with file-based routing
- **API Client**: OpenAPI-generated TypeScript client

## Deployment

- **Docker**: Multi-stage Dockerfile for optimized builds
- **Environment**: Uses `.env` file for sensitive configuration
- **Health checks**: Built-in health endpoints for monitoring
