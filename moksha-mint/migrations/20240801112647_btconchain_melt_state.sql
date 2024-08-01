-- Add migration script here
ALTER TABLE onchain_melt_quotes
ADD COLUMN state TEXT;

UPDATE onchain_melt_quotes
SET state = CASE
    WHEN paid = true THEN 'PAID'
    WHEN paid = false THEN 'UNPAID'
END;

ALTER TABLE onchain_melt_quotes
DROP COLUMN paid;

ALTER TABLE onchain_melt_quotes
ALTER COLUMN state SET NOT NULL;