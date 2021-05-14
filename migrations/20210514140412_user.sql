-- Add migration script here

CREATE TABLE user (
    id SERIAL PRIMARY KEY,
    l1_address VARCHAR(64) NOT NULL DEFAULT '',
    l2_address VARCHAR(64) NOT NULL DEFAULT '',
    UNIQUE (l1_address),
    UNIQUE (l2_address)
);

-- CREATE UNIQUE INDEX user_l1_address ON user (l1_address);
-- CREATE UNIQUE INDEX user_l2_address ON user (l2_address);
