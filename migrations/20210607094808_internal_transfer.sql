-- Add migration script here
CREATE TABLE internal_tx (
    time TIMESTAMP(0) NOT NULL, -- TODO: how is this used?
    user_from INT CHECK (user_id >= 0) NOT NULL,
    user_to INT CHECK (user_id >= 0) NOT NULL,
    asset VARCHAR(30) NOT NULL REFERENCES asset(id),
    amount DECIMAL(30, 8) NOT NULL
);

CREATE INDEX internal_tx_idx_to_time ON internal_tx (user_to, time DESC);
CREATE INDEX internal_tx_idx_from_time ON internal_tx (user_from, time DESC);

SELECT create_hypertable('internal_tx', 'time');
