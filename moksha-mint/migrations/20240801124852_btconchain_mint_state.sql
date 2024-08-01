-- Add migration script here
ALTER TABLE onchain_mint_quotes
ADD COLUMN state TEXT;

UPDATE onchain_mint_quotes
SET state = CASE
    WHEN paid = true THEN 'PAID'
    WHEN paid = false THEN 'UNPAID'
END;

ALTER TABLE onchain_mint_quotes
DROP COLUMN paid;

ALTER TABLE onchain_mint_quotes
ALTER COLUMN state SET NOT NULL;