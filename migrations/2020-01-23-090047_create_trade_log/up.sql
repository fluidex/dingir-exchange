-- Your SQL goes here
CREATE TABLE `balance_slice` (
    `id`            INT UNSIGNED NOT NULL PRIMARY KEY AUTO_INCREMENT,
    `slice_id`      BIGINT NOT NULL,
    `user_id`       INT UNSIGNED NOT NULL,
    `asset`         VARCHAR(30) NOT NULL,
    `t`             TINYINT UNSIGNED NOT NULL,
    `balance`       DECIMAL(30,16) NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8;

CREATE TABLE `order_slice` (
    `id`            BIGINT UNSIGNED NOT NULL,
    `slice_id`      BIGINT NOT NULL,
    `t`             TINYINT UNSIGNED NOT NULL,
    `side`          TINYINT UNSIGNED NOT NULL,
    `create_time`   TIMESTAMP NOT NULL,
    `update_time`   TIMESTAMP NOT NULL,
    `user_id`       INT UNSIGNED NOT NULL,
    `market`        VARCHAR(30) NOT NULL,
    `price`         DECIMAL(30,8) NOT NULL,
    `amount`        DECIMAL(30,8) NOT NULL,
    `taker_fee`     DECIMAL(30,4) NOT NULL,
    `maker_fee`     DECIMAL(30,4) NOT NULL,
    `left`          DECIMAL(30,8) NOT NULL,
    `freeze`        DECIMAL(30,8) NOT NULL,
    `deal_stock`    DECIMAL(30,8) NOT NULL,
    `deal_money`    DECIMAL(30,16) NOT NULL,
    `deal_fee`      DECIMAL(30,12) NOT NULL,
     PRIMARY KEY(slice_id, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8;

CREATE TABLE `slice_history` (
    `id`            INT UNSIGNED NOT NULL PRIMARY KEY AUTO_INCREMENT,
    `time`          BIGINT NOT NULL,
    `end_operation_log_id`   BIGINT UNSIGNED NOT NULL,
    `end_order_id`  BIGINT UNSIGNED NOT NULL,
    `end_deal_id`  BIGINT UNSIGNED NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8;

CREATE TABLE `operation_log` (
    `id`            BIGINT UNSIGNED NOT NULL PRIMARY KEY,
    `time`          TIMESTAMP NOT NULL,
    `method`        TEXT NOT NULL,
    `params`        TEXT NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8;
