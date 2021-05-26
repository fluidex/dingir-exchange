-- Add migration script here

CREATE TABLE account (
    id BIGINT CHECK (id >= 0) NOT NULL PRIMARY KEY, -- need to be consistent with rollup account_id
    l1_address VARCHAR(64) NOT NULL DEFAULT '',
    l2_pubkey VARCHAR(64) NOT NULL DEFAULT ''
);

CREATE INDEX account_l1_address ON account (l1_address);
CREATE INDEX account_l2_pubkey ON account (l2_pubkey);
