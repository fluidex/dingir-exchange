-- Add migration script here
CREATE UNIQUE INDEX market_name_constraint ON market (market_name)
WHERE
    market_name IS NOT NULL;

CREATE UNIQUE INDEX market_pair_constraint_when_null ON market (base_asset, quote_asset)
WHERE
    market_name IS NULL;

