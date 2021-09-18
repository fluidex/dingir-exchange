CREATE TABLE balance_history (
    id SERIAL PRIMARY KEY,
    time TIMESTAMP(0) NOT NULL,
    user_id INT CHECK (user_id >= 0) NOT NULL,
    business_id BIGINT CHECK (business_id >= 0) NOT NULL,
    asset VARCHAR(30) NOT NULL,
    business VARCHAR(30) NOT NULL,
    change DECIMAL(30, 8) NOT NULL,
    balance DECIMAL(30, 16) NOT NULL,
    balance_available DECIMAL(30, 16) NOT NULL,
    balance_frozen DECIMAL(30, 16) NOT NULL,
    detail TEXT NOT NULL,
    signature BYTEA NOT NULL
);

CREATE INDEX balance_history_idx_user_asset ON balance_history (user_id, asset);

CREATE INDEX balance_history_idx_user_business ON balance_history (business_id, business);

CREATE INDEX balance_history_idx_user_asset_business ON balance_history (user_id, business_id, asset, business);

CREATE TYPE order_status AS ENUM('active','filled','cancelled', 'expired');

CREATE TABLE order_history (
    id BIGINT CHECK (id >= 0) NOT NULL PRIMARY KEY,
    create_time TIMESTAMP(0) NOT NULL,
    finish_time TIMESTAMP(0) NOT NULL,
    user_id INT CHECK (user_id >= 0) NOT NULL,
    market VARCHAR(30) NOT NULL,
    order_type VARCHAR(30) NOT NULL,
    order_side VARCHAR(30) NOT NULL,
    price DECIMAL(30, 8) NOT NULL,
    amount DECIMAL(30, 8) NOT NULL,
    taker_fee DECIMAL(30, 4) NOT NULL,
    maker_fee DECIMAL(30, 4) NOT NULL,
    finished_base DECIMAL(30, 8) NOT NULL,
    finished_quote DECIMAL(30, 16) NOT NULL,
    finished_fee DECIMAL(30, 16) NOT NULL,
    status order_status NOT NULL DEFAULT 'filled',
    post_only BOOL NOT NULL DEFAULT 'false',
    signature BYTEA NOT NULL
);

CREATE INDEX order_history_idx_user_market ON order_history (user_id, market);

CREATE TABLE user_trade (
    id SERIAL PRIMARY KEY,
    time TIMESTAMP(0) NOT NULL,
    user_id INT CHECK (user_id >= 0) NOT NULL,
    market VARCHAR(30) NOT NULL,
    trade_id BIGINT CHECK (trade_id >= 0) NOT NULL,
    order_id BIGINT CHECK (order_id >= 0) NOT NULL,
    counter_order_id BIGINT CHECK (counter_order_id >= 0) NOT NULL,
    side SMALLINT CHECK (side >= 0) NOT NULL,
    role SMALLINT CHECK (ROLE >= 0) NOT NULL,
    price DECIMAL(30, 8) NOT NULL,
    amount DECIMAL(30, 8) NOT NULL,
    quote_amount DECIMAL(30, 16) NOT NULL,
    fee DECIMAL(30, 16) NOT NULL,
    counter_order_fee DECIMAL(30, 16) NOT NULL
);

CREATE INDEX user_trade_idx_user_market ON user_trade (user_id, market);

