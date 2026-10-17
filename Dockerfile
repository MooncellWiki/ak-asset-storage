# syntax=docker/dockerfile:1

FROM node:22-trixie-slim AS frontend-deps
ENV CI=true
RUN corepack enable
WORKDIR /app
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY .husky/install.mjs .husky/install.mjs
RUN pnpm install --frozen-lockfile

FROM frontend-deps AS frontend-build-stage
COPY app ./app
COPY public ./public
COPY index.html tsconfig*.json uno.config.ts vite.config.ts ./
RUN pnpm run build

FROM rust:1.97.1-trixie AS chef
RUN cargo install cargo-chef --version 0.1.78 --locked
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS build-stage
ENV SQLX_OFFLINE=true
COPY rust-toolchain.toml ./
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY .sqlx ./.sqlx
COPY --from=frontend-build-stage /app/dist /app/dist
RUN cargo build --release --locked --bin ak-asset-storage

FROM debian:13-slim
# plocate provides `updatedb`/`plocate` for the asset search index
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3t64 plocate \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build-stage /app/target/release/ak-asset-storage /app
