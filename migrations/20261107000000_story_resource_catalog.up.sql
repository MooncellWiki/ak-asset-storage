-- One row per listing resource. Rebuilt with the usage snapshot, never per request.
CREATE TABLE story_resource_catalog (
    resource_type TEXT NOT NULL,
    id TEXT NOT NULL,
    base TEXT,
    category TEXT,
    script_count BIGINT NOT NULL,
    group_script_count BIGINT NOT NULL,
    aliases TEXT[] NOT NULL,
    sort_key TEXT[] COLLATE "C" NOT NULL,
    PRIMARY KEY (resource_type, id)
);
CREATE INDEX story_resource_catalog_base_idx ON story_resource_catalog (base)
    WHERE base IS NOT NULL;

-- Numeric runs compare by significant length, then value (no integer overflow).
-- Equal natural keys are ordered by the original id by callers.
CREATE FUNCTION story_resource_sort_key(value TEXT) RETURNS TEXT[]
LANGUAGE SQL IMMUTABLE STRICT PARALLEL SAFE AS $$
    SELECT array_agg(CASE WHEN token[1] ~ '^[0-9]'
        THEN 'n' || lpad(length(ltrim(token[1], '0'))::text, 10, '0') || ltrim(token[1], '0')
        ELSE 't' || token[1] END ORDER BY ordinal)
    FROM regexp_matches(lower(value), '([0-9]+|[^0-9]+)', 'g') WITH ORDINALITY AS parts(token, ordinal)
$$;

CREATE FUNCTION story_refresh_resource_catalog(extra_aliases JSONB DEFAULT '{}') RETURNS VOID
LANGUAGE SQL AS $$
    DELETE FROM story_resource_catalog;
    INSERT INTO story_resource_catalog
    WITH normalized AS MATERIALIZED (
        SELECT *, CASE WHEN resource_type = 'character'
            THEN lower(regexp_replace(split_part(listing_id, '#', 1), '\$[0-9]+$', ''))
            ELSE NULL END AS base
        FROM story_resource_usages
    ), groups AS (
        SELECT base, count(DISTINCT script_path) AS scripts
        FROM normalized WHERE base IS NOT NULL GROUP BY base
    ), extra AS MATERIALIZED (
        SELECT value AS listing_id, array_agg(key) AS aliases
        FROM jsonb_each_text(extra_aliases) GROUP BY value
    ), listings AS (
        SELECT resource_type, listing_id, base, count(DISTINCT script_path) AS scripts,
            array_agg(DISTINCT resource_id) AS aliases
        FROM normalized GROUP BY resource_type, listing_id, base
    )
    SELECT l.resource_type, l.listing_id, l.base,
        CASE WHEN l.base IS NULL THEN NULL
             WHEN l.base ~ '^(avg_|avgnew_)?npc_' THEN 'npc'
             WHEN l.base ~ '^(char_|avg_|avgnew_|avg_char_)[0-9]' THEN 'operator'
             ELSE 'other' END,
        l.scripts, coalesce(g.scripts, l.scripts),
        ARRAY(SELECT DISTINCT lower(alias)
            FROM unnest(l.aliases || coalesce(e.aliases, '{}'::text[])) AS alias),
        story_resource_sort_key(l.listing_id)
    FROM listings l LEFT JOIN groups g USING (base)
        LEFT JOIN extra e ON l.resource_type = 'character' AND e.listing_id = l.listing_id;
$$;

-- Existing installations can serve the new interface immediately.
SELECT story_refresh_resource_catalog();
