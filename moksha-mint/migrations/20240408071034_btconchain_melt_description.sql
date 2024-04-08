-- Add the column without the NOT NULL constraint
ALTER TABLE onchain_melt_quotes
ADD COLUMN description text;

-- Set all existing rows to an empty string
UPDATE onchain_melt_quotes
SET description = '';

-- Add the NOT NULL constraint
ALTER TABLE onchain_melt_quotes
ALTER COLUMN description SET NOT NULL;