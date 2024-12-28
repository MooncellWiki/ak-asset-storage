-- Add migration script here
CREATE TABLE IF NOT EXISTS versions (
    id SERIAL PRIMARY KEY,
    res VARCHAR(32),
    client VARCHAR(32),
    is_ready BOOLEAN,
    hot_update_list TEXT
);

CREATE TABLE IF NOT EXISTS files (
    id SERIAL PRIMARY KEY,
    hash CHAR(64) UNIQUE,
    size INTEGER
);

CREATE TABLE IF NOT EXISTS bundles (
    id SERIAL PRIMARY KEY,
    path VARCHAR(256),
    version INTEGER,
    file INTEGER,
    FOREIGN KEY (version) REFERENCES versions(id),
    FOREIGN KEY (file) REFERENCES files(id),
    UNIQUE (path, version, file)
);
