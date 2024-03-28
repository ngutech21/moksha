-- Add migration script here
CREATE TABLE IF NOT EXISTS proofs (
                keyset_id TEXT,
                amount INTEGER NOT NULL,
                C TEXT NOT NULL,
                secret TEXT NOT NULL, 
                time_created TIMESTAMP, 
                UNIQUE (secret)
);

CREATE TABLE IF NOT EXISTS keysets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mint_url TEXT NOT NULL,
                keyset_id TEXT NOT NULL,
                currency_unit TEXT NOT NULL,
                active BOOL DEFAULT TRUE,
                last_index INTEGER,
                public_keys JSON CHECK (json_valid(public_keys)),
                UNIQUE (keyset_id, mint_url)
);