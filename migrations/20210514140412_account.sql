-- Add migration script here

CREATE TABLE account (
    id SERIAL PRIMARY KEY,
    l1_address VARCHAR(64) NOT NULL DEFAULT '',
    l2_address VARCHAR(64) NOT NULL DEFAULT ''
);

CREATE INDEX account_l1_address ON account (l1_address);
CREATE INDEX account_l2_address ON account (l2_address);
