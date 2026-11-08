#!/usr/bin/env bash
# Only for fresh, disposable build databases. Application databases use SQLx's
# migration runner and its migration history instead.
set -euo pipefail
: "${DATABASE_URL:?Set DATABASE_URL to a fresh build database}"
for migration in migrations/*.up.sql; do
  psql "$DATABASE_URL" -X -v ON_ERROR_STOP=1 -f "$migration"
done
