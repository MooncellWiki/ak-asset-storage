-- An interrupted CONCURRENTLY build leaves the index INVALID while the
-- creating migration stays unapplied; IF NOT EXISTS then skips the rebuild
-- by name and the unusable index would slip through unnoticed. Require a
-- valid index here so a missing or invalid one fails loudly instead.
-- Remedy: `sqlx migrate revert` (the index migration's down drops the
-- leftover), then `sqlx migrate run` to rebuild.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_index i
        JOIN pg_class c ON c.oid = i.indexrelid
        WHERE c.relname = 'bundles_version_path_id_idx'
          AND i.indisvalid
    ) THEN
        RAISE EXCEPTION 'bundles_version_path_id_idx is missing or invalid (dropped, or an interrupted build?): run "sqlx migrate revert" then "sqlx migrate run" to rebuild it';
    END IF;
END $$;
