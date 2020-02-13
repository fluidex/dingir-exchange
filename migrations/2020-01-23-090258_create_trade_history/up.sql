-- Your SQL goes here
CREATE TABLE `balance_history` (
    `id`            BIGINT UNSIGNED NOT NULL PRIMARY KEY AUTO_INCREMENT,
    `time`          TIMESTAMP NOT NULL,
    `user_id`       INT UNSIGNED NOT NULL,
    `asset`         VARCHAR(30) NOT NULL,
    `business`      VARCHAR(30) NOT NULL,
    `change`        DECIMAL(30,8) NOT NULL,
    `balance`       DECIMAL(30,16) NOT NULL,
    `detail`        TEXT NOT NULL,
    INDEX `idx_user_asset` (`user_id`, `asset`),
    INDEX `idx_user_asset_business` (`user_id`, `asset`, `business`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8;

CREATE TABLE `order_history` (
    `id`            BIGINT UNSIGNED NOT NULL PRIMARY KEY,
    `create_time`   TIMESTAMP NOT NULL,
    `finish_time`   TIMESTAMP NOT NULL,
    `user_id`       INT UNSIGNED NOT NULL,
    `market`        VARCHAR(30) NOT NULL,
    `t`             TINYINT UNSIGNED NOT NULL,
    `side`          TINYINT UNSIGNED NOT NULL,
    `price`         DECIMAL(30,8) NOT NULL,
    `amount`        DECIMAL(30,8) NOT NULL,
    `taker_fee`     DECIMAL(30,4) NOT NULL,
    `maker_fee`     DECIMAL(30,4) NOT NULL,
    `finished_base`    DECIMAL(30,8) NOT NULL,
    `finished_quote`    DECIMAL(30,16) NOT NULL,
    `finished_fee`      DECIMAL(30,16) NOT NULL,
    INDEX `idx_user_market` (`user_id`, `market`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8;

CREATE TABLE `trade_history` (
    `id`            BIGINT UNSIGNED NOT NULL PRIMARY KEY AUTO_INCREMENT,
    `time`          TIMESTAMP NOT NULL,
    `user_id`       INT UNSIGNED NOT NULL,
    `market`        VARCHAR(30) NOT NULL,
    `trade_id`       BIGINT UNSIGNED NOT NULL,
    `order_id`      BIGINT UNSIGNED NOT NULL,
    `counter_order_id` BIGINT UNSIGNED NOT NULL,
    `side`          TINYINT UNSIGNED NOT NULL,
    `role`          TINYINT UNSIGNED NOT NULL,
    `price`         DECIMAL(30,8) NOT NULL,
    `amount`        DECIMAL(30,8) NOT NULL,
    `quote_amount`          DECIMAL(30,16) NOT NULL,
    `fee`           DECIMAL(30,16) NOT NULL,
    `counter_order_fee`      DECIMAL(30,16) NOT NULL,
    INDEX `idx_user_market` (`user_id`, `market`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8;
