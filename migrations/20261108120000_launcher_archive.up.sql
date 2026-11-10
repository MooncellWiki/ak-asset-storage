-- Launcher files are raw bytes, independent of Android's ZIP content hashes.
CREATE TABLE launcher_releases (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    package_version text NOT NULL,
    game_version text NOT NULL,
    file_path text NOT NULL,
    manifest_md5 text NOT NULL CHECK (manifest_md5 ~ '^[0-9a-f]{32}$'),
    encrypted_manifest bytea NOT NULL,
    discovered_at timestamptz NOT NULL DEFAULT now(),
    ready_at timestamptz,
    UNIQUE (package_version, manifest_md5)
);

CREATE TABLE launcher_blobs (
    md5 text NOT NULL CHECK (md5 ~ '^[0-9a-f]{32}$'),
    size bigint NOT NULL CHECK (size >= 0),
    sha256 text CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    attempts integer NOT NULL DEFAULT 0,
    retry_at timestamptz NOT NULL DEFAULT now(),
    last_error text,
    PRIMARY KEY (md5, size)
);

CREATE TABLE launcher_entries (
    release_id bigint NOT NULL REFERENCES launcher_releases ON DELETE CASCADE,
    path text NOT NULL,
    md5 text NOT NULL,
    size bigint NOT NULL,
    PRIMARY KEY (release_id, path),
    FOREIGN KEY (md5, size) REFERENCES launcher_blobs (md5, size)
);
CREATE INDEX launcher_entries_content_idx ON launcher_entries (md5, size, release_id DESC);
CREATE INDEX launcher_blobs_pending_idx ON launcher_blobs (retry_at) WHERE sha256 IS NULL;

-- Numeric parts of a package version, so releases order by version (79.0.0
-- after 9.1.0) instead of by insertion: historical imports get higher ids.
CREATE FUNCTION launcher_version_key(version text) RETURNS bigint[]
LANGUAGE sql IMMUTABLE PARALLEL SAFE
RETURN ARRAY(
    SELECT part[1]::bigint
    FROM regexp_matches(version, '\d{1,18}', 'g') WITH ORDINALITY AS m(part, position)
    ORDER BY position
);

CREATE VIEW launcher_release_progress AS
SELECT r.id, r.package_version, r.game_version, r.file_path, r.manifest_md5,
       r.discovered_at, r.ready_at,
       count(e.path) AS total_files,
       count(b.sha256) AS archived_files,
       coalesce(sum(e.size), 0)::bigint AS total_bytes,
       coalesce(sum(e.size) FILTER (WHERE b.sha256 IS NOT NULL), 0)::bigint AS archived_bytes
FROM launcher_releases r
LEFT JOIN launcher_entries e ON e.release_id = r.id
LEFT JOIN launcher_blobs b ON (b.md5, b.size) = (e.md5, e.size)
GROUP BY r.id;
