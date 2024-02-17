-- Add migration script here
CREATE TABLE IF NOT EXISTS onchain_mint_quotes
(
    id uuid NOT NULL,
    address text COLLATE pg_catalog."default" NOT NULL,
	amount bigint NOT NULL,
    expiry bigint NOT NULL,
    paid boolean NOT NULL,
    CONSTRAINT onchain_mint_quotes_pkey PRIMARY KEY (id)
);


CREATE TABLE IF NOT EXISTS onchain_melt_quotes
(
    id uuid NOT NULL,
	amount bigint NOT NULL,
    address text COLLATE pg_catalog."default" NOT NULL,
    fee_total bigint NOT NULL,
    fee_sat_per_vbyte bigint NOT NULL,
    expiry bigint NOT NULL,
    paid boolean NOT NULL,
    CONSTRAINT onchain_melt_quotes_pkey PRIMARY KEY (id)
);
