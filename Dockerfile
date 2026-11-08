# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e

# The frontend is built outside the image and passed in as the `frontend`
# build context, which replaces this stage:
#   pnpm build && docker buildx build --build-context frontend=dist .
# Release uploads that same build to OSS, so the image embeds the uploaded files.
FROM debian:13-slim@sha256:62bd423aeee912639ca8ee681c2b762cb3e25137e0a92b20be455898a9f034ad AS frontend
RUN echo 'Build the frontend with `pnpm build`, then pass --build-context frontend=dist' >&2 \
    && exit 1

FROM lukemathwalker/cargo-chef:0.1.78-rust-1.99.1-trixie@sha256:cccf15fc75944521a3aa230fe72a101ee0edc65cd1d4b50b5914ce25317c1805 AS chef
ENV RUSTUP_AUTO_INSTALL=0
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS build-stage
ENV SQLX_OFFLINE=true
# Validate queries against a disposable local PostgreSQL during the final
# build, keeping the checked-in .sqlx directory untouched.
RUN apt-get update \
    && apt-get install -y --no-install-recommends postgresql \
    && rm -rf /var/lib/apt/lists/*
# Line tables and function names are enough for Sentry symbolication. Set here,
# not in Cargo.toml, so other release builds keep Cargo's default stripping.
ENV CARGO_PROFILE_RELEASE_DEBUG=line-tables-only
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json
COPY . .
COPY --from=frontend / /app/dist
ARG BUILD_SHA=dev
RUN pg_virtualenv sh -ec '\
    export DATABASE_URL="postgresql://${PGUSER}:${PGPASSWORD}@${PGHOST}:${PGPORT}/${PGDATABASE}"; \
    bash .github/scripts/migrate-build-database.sh; \
    SQLX_OFFLINE=false BUILD_SHA="$1" cargo build --release --locked --bin ak-asset-storage \
    ' sh "${BUILD_SHA}"

# Split the debug info from the binary the runtime image ships. Workflows check
# the exported file against that binary with .github/scripts/check-sentry-debug-file.sh.
RUN mkdir -p /debug \
    && objcopy --only-keep-debug target/release/ak-asset-storage /debug/ak-asset-storage.debug \
    && objcopy --strip-debug target/release/ak-asset-storage \
    && objcopy --add-gnu-debuglink=/debug/ak-asset-storage.debug target/release/ak-asset-storage

FROM scratch AS debug-files
COPY --from=build-stage /debug/ /

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
