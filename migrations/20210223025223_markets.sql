-- Add migration script here
CREATE TABLE asset (
    symbol VARCHAR(30) NOT NULL PRIMARY KEY,
    name VARCHAR(30) NOT NULL DEFAULT '',
    chain_id SMALLINT CHECK (chain_id >= 0) NOT NULL DEFAULT 1,
    token_address VARCHAR(64) NOT NULL DEFAULT '',
    precision_stor SMALLINT CHECK (precision_stor >= 0) NOT NULL,
    precision_show SMALLINT CHECK (precision_show >= 0) NOT NULL,
    logo_uri VARCHAR(256) NOT NULL DEFAULT '',
    create_time TIMESTAMP(0) DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE market (
    id SERIAL PRIMARY KEY,
    create_time TIMESTAMP(0) DEFAULT CURRENT_TIMESTAMP,
    base_asset VARCHAR(30) NOT NULL REFERENCES asset(symbol) ON DELETE RESTRICT,
    quote_asset VARCHAR(30) NOT NULL REFERENCES asset(symbol) ON DELETE RESTRICT,
    precision_base SMALLINT CHECK (precision_base >= 0) NOT NULL,
    precision_quote SMALLINT CHECK (precision_quote >= 0) NOT NULL,
    precision_fee SMALLINT CHECK (precision_fee >= 0) NOT NULL,
    min_amount DECIMAL(16, 16) NOT NULL,
    market_name VARCHAR(30)
);


