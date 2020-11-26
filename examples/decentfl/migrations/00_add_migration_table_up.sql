CREATE TABLE IF NOT EXISTS migrations (
    id              INTEGER PRIMARY KEY,
    migration_id    INTEGER,
    success         BOOLEAN
);