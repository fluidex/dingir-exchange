-- Add migration script here
CREATE TABLE asset (
    symbol VARCHAR(30) NOT NULL DEFAULT '',
    name VARCHAR(30) NOT NULL DEFAULT '',
    chain_id SMALLINT DEFAULT 1,
    token_address VARCHAR(64) NOT NULL,
    is_commonly_quoted BOOLEAN DEFAULT false,
    precision_stor SMALLINT CHECK (precision_stor >= 0) NOT NULL,
    precision_show SMALLINT CHECK (precision_show >= 0) NOT NULL,
    logo_uri VARCHAR(256) NOT NULL DEFAULT '',
    create_time TIMESTAMP(0) DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (chainId, token_address)
);

CREATE INDEX asset_idx_symbol ON asset (symbol);
CREATE INDEX asset_idx_token_address ON asset (token_address);

-- TODO: need to remove this table?
CREATE TABLE market (
    id SERIAL PRIMARY KEY,
    create_time TIMESTAMP(0) DEFAULT CURRENT_TIMESTAMP,
    base_asset VARCHAR(30) NOT NULL REFERENCES asset(symbol) ON DELETE RESTRICT, -- TODO: USE address
    quote_asset VARCHAR(30) NOT NULL REFERENCES asset(symbol) ON DELETE RESTRICT, -- TODO: USE address
    precision_base SMALLINT CHECK (precision_base >= 0) NOT NULL,
    precision_quote SMALLINT CHECK (precision_quote >= 0) NOT NULL,
    precision_fee SMALLINT CHECK (precision_fee >= 0) NOT NULL,
    min_amount DECIMAL(16, 16) NOT NULL,
    market_name VARCHAR(30)
);


