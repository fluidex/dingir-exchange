#![allow(unused_imports)]
#![allow(clippy::single_component_path_imports)]

table! {
    operation_log(id) {
        id -> Unsigned<Bigint>,
        time -> Timestamp,
        method -> Text,
        params -> Text,
    }
}

// history
table! {
    balance_history (id) {
        id -> Unsigned<Bigint>,
        time -> Timestamp,
        user_id -> Unsigned<Integer>,
        asset -> Varchar,
        business -> Varchar,
        change -> Decimal,
        balance -> Decimal,
        detail -> Text,
    }
}

table! {
    order_history (id) {
        id -> Unsigned<Bigint>,
        create_time -> Timestamp,
        finish_time -> Timestamp,
        user_id -> Unsigned<Integer>,
        market -> Varchar,
        t -> Unsigned<Tinyint>,
        side -> Unsigned<Tinyint>,
        price -> Decimal,
        amount -> Decimal,
        taker_fee -> Decimal,
        maker_fee -> Decimal,
        finished_base -> Decimal,
        finished_quote -> Decimal,
        finished_fee -> Decimal,
    }
}

table! {
    trade_history (id) {
        id -> Unsigned<Bigint>,
        time -> Timestamp,
        user_id -> Unsigned<Integer>,
        market -> Varchar,
        trade_id -> Unsigned<Bigint>,
        order_id -> Unsigned<Bigint>,
        counter_order_id -> Unsigned<Bigint>,
        side -> Unsigned<Tinyint>,
        role -> Unsigned<Tinyint>,
        price -> Decimal,
        amount -> Decimal,
        quote_amount -> Decimal,
        fee -> Decimal,
        counter_order_fee -> Decimal,
    }
}

// slice
table! {
    slice_history (id) {
        id -> Unsigned<Integer>,
        time -> Bigint,
        end_operation_log_id -> Unsigned<Bigint>,
        end_order_id -> Unsigned<Bigint>,
        end_trade_id -> Unsigned<Bigint>,
    }
}

table! {
    balance_slice (id) {
        id -> Unsigned<Integer>,
        slice_id -> Bigint,
        user_id -> Unsigned<Integer>,
        asset -> Varchar,
        t -> Unsigned<Tinyint>,
        balance -> Decimal,
    }
}

table! {
    order_slice (slice_id, id) {
        id -> Unsigned<Bigint>,
        slice_id -> Bigint,
        t -> Unsigned<Tinyint>,
        side -> Unsigned<Tinyint>,
        create_time -> Timestamp,
        update_time -> Timestamp,
        user_id -> Unsigned<Integer>,
        market -> Varchar,
        price -> Decimal,
        amount -> Decimal,
        taker_fee -> Decimal,
        maker_fee -> Decimal,
        left -> Decimal,
        freeze -> Decimal,
        finished_base -> Decimal,
        finished_quote -> Decimal,
        finished_fee -> Decimal,
    }
}

allow_tables_to_appear_in_same_query!(
    balance_history,
    order_history,
    trade_history,
    operation_log,
    balance_slice,
    order_slice,
    slice_history,
);
