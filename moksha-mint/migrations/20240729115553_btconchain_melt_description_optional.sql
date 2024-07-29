-- Add migration script here
ALTER TABLE onchain_melt_quotes
ALTER COLUMN description DROP NOT NULL;