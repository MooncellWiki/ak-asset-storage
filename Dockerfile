# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e

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

FROM lukemathwalker/cargo-chef:0.1.78-rust-1.99.1-trixie@sha256:cccf15fc75944521a3aa230fe72a101ee0edc65cd1d4b50b5914ce25317c1805 AS chef
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

FROM debian:13-slim@sha256:62bd423aeee912639ca8ee681c2b762cb3e25137e0a92b20be455898a9f034ad
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
