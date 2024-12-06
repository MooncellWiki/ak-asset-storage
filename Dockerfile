FROM node:20-slim as frontend-build-stage
RUN corepack enable
WORKDIR /app
COPY . /app
RUN pnpm i
RUN pnpm run build:ui

FROM rust:1.83-bookworm as build-stage
WORKDIR /app
COPY . /app/
COPY --from=frontend-build-stage /app/packages/ui/dist /app/packages/ui/dist
RUN cd packages/ui/dist && ls -la
RUN cargo build --all --release

FROM debian:bookworm
RUN apt-get update && apt-get -y install ca-certificates
WORKDIR /app
COPY --from=build-stage /app/target/release/ak_asset_storage /app
