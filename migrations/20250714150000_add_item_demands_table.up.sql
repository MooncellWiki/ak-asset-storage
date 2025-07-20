-- Add item_demands table for v2 schema
CREATE TABLE IF NOT EXISTS item_demands (
    name VARCHAR(32) PRIMARY KEY NOT NULL,
    usage TEXT NOT NULL
);

-- Create index on name for better query performance (though it's already PK)
CREATE INDEX IF NOT EXISTS item_demands_name_idx ON item_demands(name);
