# Arknights Asset Storage

This project is a storage service for Arknights assets. It consists of a Rust-based backend and a Vue-based frontend.

## Project Structure

The project is a monorepo containing both the backend and frontend.

- `crates/`: This directory contains the Rust workspace and its crates.
  - `application/`: Contains the core application logic, following a clean architecture pattern.
    - `dto/`: Data Transfer Objects for moving data between layers.
    - `entities/`: Core business entities.
    - `ports/`: Interfaces for external services (e.g., repositories, services).
    - `services/`: Application services that orchestrate business logic.
    - `value_objects/`: Value objects used in the domain.
  - `cli/`: A command-line interface for the application.
    - `commands/`: Defines the available CLI commands.
  - `infrastructure/`: Contains the infrastructure logic, such as database access and external service integrations.
    - `config/`: Application configuration.
    - `persistence/`: Database access logic (repositories).
    - `external/`: Integrations with external services.
    - `scheduling/`: Scheduled tasks.
  - `web/`: The web server, built with Axum.
    - `handlers/`: HTTP request handlers.
    - `routes.rs`: Defines the application's routes.
    - `middleware.rs`: Custom middleware.
    - `server.rs`: Server setup and startup.
- `app/`: This directory contains the Vue frontend application.
- `migrations/`: Contains SQL database migration files.
- `docs/`: Contains documentation for the project.
- `public/`: Contains static assets for the frontend.
