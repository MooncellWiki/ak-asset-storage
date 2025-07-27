# Contributing to Arknights Asset Storage

Thank you for your interest in contributing to the Arknights Asset Storage project! This guide will help you set up your development environment.

## Prerequisites

- Rust (latest stable)
- Node.js (v20 or higher)
- Docker and Docker Compose
- Git

## Development Setup

### 1. Install Required Tools

First, install `cargo-binstall` and `just`:

```bash
cargo install cargo-binstall
cargo binstall just -y
```

### 2. Initialize the Project

Run the initialization script to set up dependencies:

```bash
just init
```

### 3. Install Frontend Dependencies

```bash
pnpm install
```

### 4. Start Infrastructure Services

Start the required services (PostgreSQL and MinIO):

```bash
docker compose up -d
```

### 5. Run Database Migrations

```bash
sqlx migrate run
```

## Development Workflow

### Backend Development

Start the backend API server:

```bash
cargo run --bin ak-asset-storage -- server -c config.toml
```

Start the background worker:

```bash
cargo run --bin ak-asset-storage -- worker -c config.toml
```

### Frontend Development

Start the frontend development server:

```bash
pnpm dev
```

The frontend will be available at `http://localhost:5173` and the API at `http://localhost:3000`.

### Testing

Run backend tests:

```bash
cargo test --workspace
```

Run frontend type checking and linting:

```bash
pnpm typecheck
pnpm lint
```

### Database Management

Create a new migration:

```bash
sqlx migrate add <migration_name>
```

## Development Services

After running `docker compose up -d`, the following services will be available:

- **PostgreSQL**: `localhost:25432`
- **MinIO (S3)**: `localhost:29000` (API), `localhost:29001` (Console)
- **Frontend**: `localhost:5173` (after `pnpm dev`)
- **Backend API**: `localhost:25150` (after `cargo run --bin ak-asset-storage server`)

## Project Structure

- `crates/cli/` - Binary entry points
- `crates/web/` - HTTP API and static file serving
- `crates/application/` - Core business logic
- `crates/infrastructure/` - External service integrations
- `app/` - Vue.js frontend application

## Code Style

- Follow existing Rust code style (rustfmt)
- Follow eslint in frontend
- Write tests for new features
- Update documentation as needed
