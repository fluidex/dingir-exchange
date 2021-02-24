-- Add migration script here
--CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;
CREATE TABLE market_trade (
    time TIMESTAMP(0) NOT NULL,
    market VARCHAR(30) NOT NULL,
    trade_id BIGINT CHECK (trade_id >= 0) NOT NULL,
    price DECIMAL(30, 8) NOT NULL,
    amount DECIMAL(30, 8) NOT NULL,
    quote_amount DECIMAL(30, 8) NOT NULL,
    taker_side VARCHAR(30) NOT NULL
);

CREATE INDEX market_trade_idx_market ON market_trade (market, time DESC);

SELECT create_hypertable('market_trade', 'time');
