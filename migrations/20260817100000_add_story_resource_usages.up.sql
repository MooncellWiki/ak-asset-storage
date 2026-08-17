-- Phase 1 story resource usage lookup (docs/story-reference-phase-1-design.md)

CREATE TABLE story_dataset (
    singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE story_resource_usages (
    script_path TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    display_names TEXT[] NOT NULL DEFAULT '{}',
    sort_order INTEGER NOT NULL,

    PRIMARY KEY (script_path, resource_type, resource_id)
);

CREATE INDEX story_resource_usages_resource_idx
    ON story_resource_usages(resource_type, resource_id);

CREATE INDEX story_resource_usages_script_idx
    ON story_resource_usages(script_path, sort_order);
