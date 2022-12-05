-- Add migration script here
CREATE TABLE internal_tx (
    time TIMESTAMP(0) NOT NULL,
    user_from VARCHAR(64) NOT NULL,
    from_broker_id VARCHAR(64) NOT NULL,
    from_account_id VARCHAR(64) NOT NULL,
    user_to VARCHAR(64) NOT NULL,
    to_broker_id VARCHAR(64) NOT NULL,
    to_account_id VARCHAR(64) NOT NULL,
    asset VARCHAR(30) NOT NULL REFERENCES asset (id),
    amount DECIMAL(30, 8) CHECK (amount > 0) NOT NULL,
    signature BYTEA NOT NULL
);

CREATE INDEX internal_tx_idx_to_time ON internal_tx (user_to, time DESC);

CREATE INDEX internal_tx_idx_from_time ON internal_tx (user_from, time DESC);

SELECT
    create_hypertable ('internal_tx', 'time');

