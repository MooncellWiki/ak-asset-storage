DROP INDEX story_resource_usages_listing_idx;
CREATE INDEX story_resource_usages_listing_idx
    ON story_resource_usages(resource_type, listing_id) INCLUDE (script_path);

DROP INDEX story_resource_usages_resource_idx;
CREATE INDEX story_resource_usages_resource_idx
    ON story_resource_usages(resource_type, resource_id);

ALTER TABLE story_resource_usages
    DROP CONSTRAINT story_resource_usages_resource_type_check;
