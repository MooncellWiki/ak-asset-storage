ALTER TABLE story_resource_usages
    ADD CONSTRAINT story_resource_usages_resource_type_check
    CHECK (resource_type IN ('background', 'image', 'item', 'character'));

DROP INDEX story_resource_usages_resource_idx;
CREATE INDEX story_resource_usages_resource_idx
    ON story_resource_usages(resource_type, resource_id, script_path);

DROP INDEX story_resource_usages_listing_idx;
CREATE INDEX story_resource_usages_listing_idx
    ON story_resource_usages(resource_type, listing_id, script_path);
