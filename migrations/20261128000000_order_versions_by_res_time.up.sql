-- Resource versions start with YY-MM-DD-HH-MM-SS, then '-' or '_' before the hash.
-- Keep unrecognized formats together in ID order before timestamped versions.
ALTER TABLE versions ADD COLUMN res_time_key text COLLATE "C" GENERATED ALWAYS AS (
    COALESCE(substring(res COLLATE "C" FROM '^([0-9]{2}(?:-[0-9]{2}){5})(?:[-_]|$)'), '')
) STORED NOT NULL;

CREATE INDEX versions_res_time_id_idx ON versions (res_time_key, id);
