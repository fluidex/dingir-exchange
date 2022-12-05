-- Add migration script here
CREATE TABLE account (
    id VARCHAR(350) NOT NULL PRIMARY KEY, -- need to be consistent with rollup account_id
    broker_id VARCHAR(350) NOT NULL, -- need to be consistent with rollup account_id
    account_id VARCHAR(350) NOT NULL, -- need to be consistent with rollup account_id
    l1_address VARCHAR(350) NOT NULL DEFAULT '',
    l2_pubkey VARCHAR(350) NOT NULL DEFAULT ''
);

CREATE INDEX account_l1_address ON account (l1_address);

CREATE INDEX account_l2_pubkey ON account (l2_pubkey);

