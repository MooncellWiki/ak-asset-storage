FROM node:20-slim AS frontend-build-stage
RUN corepack enable
WORKDIR /app
COPY . /app
RUN pnpm i
RUN pnpm run build

FROM rust:1.87-bookworm AS build-stage
ENV SQLX_OFFLINE=true
WORKDIR /app
COPY . /app/
COPY --from=frontend-build-stage /app/dist /app/dist
RUN cd dist && ls -la
RUN cargo build --all --release

FROM debian:bookworm
RUN apt-get update && apt-get -y install ca-certificates
WORKDIR /app
COPY --from=build-stage /app/target/release/ak_asset_storage /app
