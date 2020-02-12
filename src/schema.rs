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
        source -> Varchar,
        t -> Unsigned<Tinyint>,
        side -> Unsigned<Tinyint>,
        price -> Decimal,
        amount -> Decimal,
        taker_fee -> Decimal,
        maker_fee -> Decimal,
        deal_stock -> Decimal,
        deal_money -> Decimal,
        deal_fee -> Decimal,
    }
}

table! {
    deal_history (id) {
        id -> Unsigned<Bigint>,
        time -> Timestamp,
        user_id -> Unsigned<Integer>,
        market -> Varchar,
        deal_id -> Unsigned<Bigint>,
        order_id -> Unsigned<Bigint>,
        deal_order_id -> Unsigned<Bigint>,
        side -> Unsigned<Tinyint>,
        role -> Unsigned<Tinyint>,
        price -> Decimal,
        amount -> Decimal,
        deal -> Decimal,
        fee -> Decimal,
        deal_fee -> Decimal,
    }
}

// slice
table! {
    slice_history (id) {
        id -> Unsigned<Integer>,
        time -> Bigint,
        end_operation_log_id -> Unsigned<Bigint>,
        end_order_id -> Unsigned<Bigint>,
        end_deal_id -> Unsigned<Bigint>,
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
        deal_stock -> Decimal,
        deal_money -> Decimal,
        deal_fee -> Decimal,
    }
}

allow_tables_to_appear_in_same_query!(
    balance_history,
    order_history,
    deal_history,
    operation_log,
    balance_slice,
    order_slice,
    slice_history,
);
