CREATE TABLE used_proofs (
    amount BIGINT NOT NULL,
    secret TEXT NOT NULL PRIMARY KEY,
    c TEXT NOT NULL,
    keyset_id TEXT NOT NULL
);

CREATE TABLE pending_invoices (
    key TEXT NOT NULL PRIMARY KEY,
    payment_request TEXT NOT NULL,
    amount BIGINT NOT NULL
);

CREATE TABLE bolt11_mint_quotes (
    id UUID PRIMARY KEY NOT NULL,
    payment_request TEXT NOT NULL,
    expiry BIGINT NOT NULL,
    paid BOOLEAN NOT NULL
);

CREATE TABLE bolt11_melt_quotes (
    id UUID PRIMARY KEY NOT NULL,
    payment_request TEXT NOT NULL,
    expiry BIGINT NOT NULL,
    paid BOOLEAN NOT NULL,
    amount BIGINT NOT NULL,
    fee_reserve BIGINT NOT NULL
);