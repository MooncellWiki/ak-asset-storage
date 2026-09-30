-- no-transaction
CREATE INDEX CONCURRENTLY IF NOT EXISTS bundles_version_path_id_idx
    ON bundles (version DESC, path ASC, id ASC);
