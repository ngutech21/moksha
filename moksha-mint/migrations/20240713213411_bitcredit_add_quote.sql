CREATE TABLE bitcredit_requests_to_mint (
    bill_id TEXT NOT NULL PRIMARY KEY,
    bill_key TEXT NOT NULL
);

CREATE TABLE bitcredit_mint_quotes (
    id UUID NOT NULL,
    bill_id TEXT NOT NULL PRIMARY KEY,
    node_id TEXT NOT NULL
);