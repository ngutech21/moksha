-- Add migration script here
CREATE TABLE seed (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    seed_words TEXT NOT NULL
    -- other columns
);