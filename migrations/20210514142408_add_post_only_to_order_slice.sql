-- Add migration script here

ALTER TABLE order_slice ADD COLUMN post_only BOOL default false;
