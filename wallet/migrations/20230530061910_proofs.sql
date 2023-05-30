-- Add migration script here
CREATE TABLE IF NOT EXISTS proofs (
                keyset_id TEXT,
                amount INTEGER NOT NULL,
                C TEXT NOT NULL,
                secret TEXT NOT NULL, 
                time_created TIMESTAMP, 
                UNIQUE (secret)
);