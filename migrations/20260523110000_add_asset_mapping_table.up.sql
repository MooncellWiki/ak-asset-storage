ALTER TABLE versions
ADD COLUMN asset_mapping_status VARCHAR(16) NOT NULL DEFAULT 'pending';

ALTER TABLE versions
ADD CONSTRAINT versions_res_unique UNIQUE (res);

CREATE TYPE node_type AS ENUM ('file', 'directory', 'both');

CREATE TABLE asset_to_bundle_mappings (
    id SERIAL PRIMARY KEY,
    version_id INTEGER NOT NULL REFERENCES versions(id),
    asset_name VARCHAR(512) NOT NULL,
    bundle_path VARCHAR(512) NOT NULL,
    asset_path VARCHAR(512),
    short_name VARCHAR(256),
    dir_name VARCHAR(512) NOT NULL DEFAULT '',
    node_type node_type NOT NULL DEFAULT 'file'
);

CREATE UNIQUE INDEX idx_atb_version_asset
    ON asset_to_bundle_mappings(version_id, asset_name);

CREATE INDEX idx_atb_version_bundle
    ON asset_to_bundle_mappings(version_id, bundle_path);

CREATE INDEX idx_atb_version_dir
    ON asset_to_bundle_mappings(version_id, dir_name);
