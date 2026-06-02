DROP INDEX IF EXISTS idx_atb_version_dir;
DROP INDEX IF EXISTS idx_atb_version_bundle;
DROP INDEX IF EXISTS idx_atb_version_asset;
DROP TABLE IF EXISTS asset_to_bundle_mappings;

DROP TYPE IF EXISTS node_type;

ALTER TABLE versions
DROP CONSTRAINT IF EXISTS versions_res_unique;

ALTER TABLE versions
DROP COLUMN IF EXISTS asset_mapping_status;

DROP TYPE IF EXISTS asset_mapping_status;
