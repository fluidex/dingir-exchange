-- Add migration script here
CREATE SEQUENCE balance_slice_seq;

CREATE TABLE balance_slice (
    id            INT CHECK (id >= 0) NOT NULL PRIMARY KEY DEFAULT NEXTVAL ('balance_slice_seq'),
    slice_id      BIGINT NOT NULL,
    user_id       INT CHECK (user_id >= 0) NOT NULL,
    asset         VARCHAR(30) NOT NULL,
    t             SMALLINT CHECK (t >= 0) NOT NULL,
    balance       DECIMAL(30,16) NOT NULL
) ;

CREATE TABLE order_slice (
    id            BIGINT CHECK (id >= 0) NOT NULL,
    slice_id      BIGINT NOT NULL,
    t             SMALLINT CHECK (t >= 0) NOT NULL,
    side          SMALLINT CHECK (side >= 0) NOT NULL,
    create_time   TIMESTAMP(0) NOT NULL,
    update_time   TIMESTAMP(0) NOT NULL,
    user_id       INT CHECK (user_id >= 0) NOT NULL,
    market        VARCHAR(30) NOT NULL,
    price         DECIMAL(30,8) NOT NULL,
    amount        DECIMAL(30,8) NOT NULL,
    taker_fee     DECIMAL(30,4) NOT NULL,
    maker_fee     DECIMAL(30,4) NOT NULL,
    remain          DECIMAL(30,8) NOT NULL,
    frozen        DECIMAL(30,8) NOT NULL,
    finished_base    DECIMAL(30,8) NOT NULL,
    finished_quote    DECIMAL(30,16) NOT NULL,
    finished_fee      DECIMAL(30,12) NOT NULL,
     PRIMARY KEY(slice_id, id)
) ;

CREATE SEQUENCE slice_history_seq;

CREATE TABLE slice_history (
    id            INT CHECK (id >= 0) NOT NULL PRIMARY KEY DEFAULT NEXTVAL ('slice_history_seq'),
    time          BIGINT NOT NULL,
    end_operation_log_id   BIGINT CHECK (end_operation_log_id >= 0) NOT NULL,
    end_order_id  BIGINT CHECK (end_order_id >= 0) NOT NULL,
    end_trade_id  BIGINT CHECK (end_trade_id >= 0) NOT NULL
) ;

CREATE TABLE operation_log (
    id            BIGINT CHECK (id >= 0) NOT NULL PRIMARY KEY,
    time          TIMESTAMP(0) NOT NULL,
    method        TEXT NOT NULL,
    params        TEXT NOT NULL
) ;
