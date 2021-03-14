-- Add migration script here

CREATE TYPE order_status AS ENUM('active','filled','cancelled', 'expired');

-- Set existed order as filled
ALTER TABLE order_history ADD status order_status NOT NULL DEFAULT 'filled';
