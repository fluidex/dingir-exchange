//#[allow(clippy::single_component_path_imports)]

table! {
    balance_history_example (id) {
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
    order_deal_history_example (id) {
        id -> Unsigned<Bigint>,
        time -> Double,
        user_id -> Unsigned<Integer>,
        deal_id -> Unsigned<Bigint>,
        order_id -> Unsigned<Bigint>,
        deal_order_id -> Unsigned<Bigint>,
        role -> Unsigned<Tinyint>,
        price -> Decimal,
        amount -> Decimal,
        deal -> Decimal,
        fee -> Decimal,
        deal_fee -> Decimal,
    }
}

table! {
    operlog_example (id) {
        id -> Unsigned<Bigint>,
        time -> Timestamp,
        method -> Text,
        params -> Text,
    }
}

table! {
    order_detail_example (id) {
        id -> Unsigned<Bigint>,
        create_time -> Double,
        finish_time -> Double,
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
    order_history_example (id) {
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
    slice_balance_example (id) {
        id -> Unsigned<Integer>,
        user_id -> Unsigned<Integer>,
        asset -> Varchar,
        t -> Unsigned<Tinyint>,
        balance -> Decimal,
    }
}

table! {
    slice_history (id) {
        id -> Unsigned<Integer>,
        time -> Bigint,
        end_oper_id -> Unsigned<Bigint>,
        end_order_id -> Unsigned<Bigint>,
        end_deals_id -> Unsigned<Bigint>,
    }
}

table! {
    slice_order_example (id) {
        id -> Unsigned<Bigint>,
        t -> Unsigned<Tinyint>,
        side -> Unsigned<Tinyint>,
        create_time -> Double,
        update_time -> Double,
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

table! {
    deal_history_example (id) {
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

allow_tables_to_appear_in_same_query!(
    balance_history_example,
    order_deal_history_example,
    operlog_example,
    order_detail_example,
    order_history_example,
    slice_balance_example,
    slice_history,
    slice_order_example,
    deal_history_example,
);
