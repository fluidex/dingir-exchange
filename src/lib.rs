#![allow(dead_code)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::let_and_return)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::single_char_pattern)]
#![allow(clippy::await_holding_refcell_ref)] // FIXME

pub mod asset;
pub mod config;
pub mod controller;
pub mod database;
pub mod dto;
pub mod history;
pub mod market;
pub mod message;
pub mod models;
pub mod persist;
pub mod restapi;
pub mod sequencer;
pub mod server;
pub mod sqlxextend;
pub mod types;
pub mod utils;
