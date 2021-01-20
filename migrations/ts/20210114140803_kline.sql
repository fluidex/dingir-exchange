-- Add migration script here
--CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;
CREATE TABLE trade_record (
    time TIMESTAMP(0) NOT NULL,
    market VARCHAR(30) NOT NULL,
    trade_id BIGINT CHECK (trade_id >= 0) NOT NULL,
    price DECIMAL(30, 8) NOT NULL,
    amount DECIMAL(30, 8) NOT NULL,
    quote_amount DECIMAL(30, 8) NOT NULL,
    taker_side VARCHAR(30) NOT NULL
);

CREATE INDEX trade_record_idx_market ON trade_record (market, time DESC);

SELECT create_hypertable('trade_record', 'time');
