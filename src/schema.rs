#![allow(unused_imports)]
#![allow(clippy::single_component_path_imports)]

table! {
    operation_log (id) {
        id -> Int8,
        time -> Timestamp,
        method -> Text,
        params -> Text,
    }
}
table! {
    balance_history (id) {
        id -> Int8,
        time -> Timestamp,
        user_id -> Int4,
        asset -> Varchar,
        business -> Varchar,
        change -> Numeric,
        balance -> Numeric,
        detail -> Text,
    }
}
table! {
    order_history (id) {
        id -> Int8,
        create_time -> Timestamp,
        finish_time -> Timestamp,
        user_id -> Int4,
        market -> Varchar,
        t -> Int2,
        side -> Int2,
        price -> Numeric,
        amount -> Numeric,
        taker_fee -> Numeric,
        maker_fee -> Numeric,
        finished_base -> Numeric,
        finished_quote -> Numeric,
        finished_fee -> Numeric,
    }
}

table! {
    trade_history (id) {
        id -> Int8,
        time -> Timestamp,
        user_id -> Int4,
        market -> Varchar,
        trade_id -> Int8,
        order_id -> Int8,
        counter_order_id -> Int8,
        side -> Int2,
        role -> Int2,
        price -> Numeric,
        amount -> Numeric,
        quote_amount -> Numeric,
        fee -> Numeric,
        counter_order_fee -> Numeric,
    }
}

table! {
    slice_history (id) {
        id -> Int4,
        time -> Int8,
        end_operation_log_id -> Int8,
        end_order_id -> Int8,
        end_trade_id -> Int8,
    }
}

table! {
    balance_slice (id) {
        id -> Int4,
        slice_id -> Int8,
        user_id -> Int4,
        asset -> Varchar,
        t -> Int2,
        balance -> Numeric,
    }
}

table! {
    order_slice (slice_id, id) {
        id -> Int8,
        slice_id -> Int8,
        t -> Int2,
        side -> Int2,
        create_time -> Timestamp,
        update_time -> Timestamp,
        user_id -> Int4,
        market -> Varchar,
        price -> Numeric,
        amount -> Numeric,
        taker_fee -> Numeric,
        maker_fee -> Numeric,
        remain -> Numeric,
        frozen -> Numeric,
        finished_base -> Numeric,
        finished_quote -> Numeric,
        finished_fee -> Numeric,
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
