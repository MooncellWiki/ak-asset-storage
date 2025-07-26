# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Arknights Asset Storage** - A Rust-based service for storing and managing Arknights game assets with a Vue.js frontend. Uses clean architecture with separate crates for different layers and PostgreSQL + S3 storage.

## Architecture

Clean architecture with 4 distinct layers:

- **CLI** (`crates/cli/`): Binary entry points for server/worker/seed commands
- **Web** (`crates/web/`): Axum HTTP API + static file serving for Vue frontend
- **Application** (`crates/application/`): Core business logic, entities, use cases
- **Infrastructure** (`crates/infrastructure/`): External services (PostgreSQL, S3, SMTP, HTTP clients)

## Quick Start

```bash
# Setup dependencies and infrastructure
just init && pnpm install
docker-compose up -d
sqlx migrate run

# Start development
pnpm dev          # Frontend (Vue + Vite)
cargo run --bin ak-asset-storage server  # Backend API
```

## Key Commands

### Backend (Rust)

```bash
cargo build --workspace          # Build all crates
cargo test --workspace          # Run all tests
cargo run --bin ak-asset-storage server  # Start web server
cargo run --bin ak-asset-storage worker  # Background sync worker
cargo run --bin ak-asset-storage seed --csv-path data.csv  # Seed DB
```

### Frontend (Vue.js)

```bash
pnpm dev        # Vite dev server with hot reload
pnpm build      # Production build
pnpm typecheck  # TypeScript checking
pnpm lint       # ESLint
pnpm api        # Regenerate API types from OpenAPI
```

### Database

```bash
sqlx migrate run        # Run migrations
sqlx migrate add <name> # Create new migration
```

## Core Domain Model

- **Bundle**: Collection of game assets (maps to game versions)
- **File**: Individual asset files with hash/version tracking
- **Version**: Game version metadata and asset relationships
- **ItemDemand**: New feature for tracking asset usage

## External Integrations

- **PostgreSQL**: Primary storage via SQLx
- **MinIO/S3**: Asset storage (configured via docker-compose)
- **Arknights API**: Asset downloads from official servers
- **SMTP**: Email notifications (optional)
- **Torappu**: Enhanced features via token-based API

## Testing Strategy

- **Unit tests**: In `crates/application/tests/unit/`
- **Integration tests**: In `crates/application/tests/integration/`
- **E2E tests**: Workflow tests with mocked external services

## Development Environment

**Infrastructure via docker-compose:**

- PostgreSQL on port 25432
- MinIO (S3) on ports 29000/29001
- Configured via `.env` file

**Frontend stack:**

- Vue 3 + TypeScript + Naive UI
- File-based routing with unplugin-vue-router
- UnoCSS for styling
- OpenAPI-generated API client

## Development Guidelines

- always run cargo check if .rs has been changed