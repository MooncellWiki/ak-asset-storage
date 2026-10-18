# syntax=docker/dockerfile:1

FROM node:24.22.0-trixie-slim@sha256:7f486cabe892bff8fef04c0f7da8dba84a560be3e4e2740b7a86afc57ebec315 AS frontend-deps
ENV CI=true
RUN corepack enable
WORKDIR /app
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY .husky/install.mjs .husky/install.mjs
RUN pnpm install --frozen-lockfile

FROM frontend-deps AS frontend-build-stage
COPY . .
RUN pnpm run build

FROM lukemathwalker/cargo-chef:0.1.78-rust-1.97.1-trixie@sha256:6dce65df3d7430c797e94348b4cf36d8d5876b63ca54f35dbfd37a97c42d0add AS chef
ENV RUSTUP_AUTO_INSTALL=0
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS build-stage
ENV SQLX_OFFLINE=true
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json
COPY . .
COPY --from=frontend-build-stage /app/dist /app/dist
ARG BUILD_SHA=dev
RUN BUILD_SHA=${BUILD_SHA} cargo build --release --locked --bin ak-asset-storage

FROM debian:13-slim@sha256:a29215f6a35e51e22adffa17f89e9d2ef06214e64a2bad10d765c46aea49f11f
# plocate provides `updatedb`/`plocate` for the asset search index
# Its postinst skips the initial full-filesystem scan if plocate.db exists.
# Remove the temporary placeholder: the app builds its own index elsewhere.
RUN mkdir -p /var/lib/plocate \
    && touch /var/lib/plocate/plocate.db \
    && apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3t64 plocate \
    && rm -f /var/lib/plocate/plocate.db \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build-stage /app/target/release/ak-asset-storage /app
