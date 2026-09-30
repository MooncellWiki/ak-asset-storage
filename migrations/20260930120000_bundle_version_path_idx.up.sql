-- no-transaction
-- Single statement on purpose: sqlx sends a no-transaction migration as one
-- batch, and any second statement would wrap this one in an implicit
-- transaction, which CONCURRENTLY forbids. An interrupted build that leaves
-- an invalid index behind is caught by the validate migration right after.
CREATE INDEX CONCURRENTLY IF NOT EXISTS bundles_version_path_id_idx
    ON bundles (version DESC, path ASC, id ASC);
