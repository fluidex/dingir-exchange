-- Add migration script here
CREATE TABLE asset (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    symbol VARCHAR(30) NOT NULL DEFAULT '',
    name VARCHAR(30) NOT NULL DEFAULT '',
    chain_id SMALLINT CHECK (chain_id >= 0) NOT NULL DEFAULT 1, -- we actually only have one same chain_id for all records
    token_address VARCHAR(64) NOT NULL DEFAULT '',
    rollup_token_id INTEGER CHECK (rollup_token_id >= 0) NOT NULL,
    -- token_address VARCHAR(64) DEFAULT NULL,
    -- UNIQUE (chain_id, token_address),
    UNIQUE (chain_id, rollup_token_id),
    precision_stor SMALLINT CHECK (precision_stor >= 0) NOT NULL,
    precision_show SMALLINT CHECK (precision_show >= 0) NOT NULL,
    logo_uri VARCHAR(256) NOT NULL DEFAULT '',
    create_time TIMESTAMP(0) DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE market (
    id SERIAL PRIMARY KEY,
    create_time TIMESTAMP(0) DEFAULT CURRENT_TIMESTAMP,
    base_asset VARCHAR(30) NOT NULL REFERENCES asset (id) ON DELETE RESTRICT,
    quote_asset VARCHAR(30) NOT NULL REFERENCES asset (id) ON DELETE RESTRICT,
    precision_amount SMALLINT CHECK (precision_amount >= 0) NOT NULL,
    precision_price SMALLINT CHECK (precision_price >= 0) NOT NULL,
    precision_fee SMALLINT CHECK (precision_fee >= 0) NOT NULL,
    min_amount DECIMAL(16, 16) NOT NULL,
    market_name VARCHAR(30)
);

