//! Gran Turismo 7 (PS5) content-store toolkit.
#![recursion_limit = "512"]
//!
//! The PS5 install stores every asset as
//! `contents/<cache_key(FNV-1a(asset path))>` with the payload already
//! decrypted.  This crate provides a parallel parser for that store, the
//! Polyphony PackedFile containers and the encrypted `.rt2` localisation
//! containers.

pub mod avatar;
pub mod census;
pub mod discover;
pub mod extract;
pub mod gpb;
pub mod gpb4;
pub mod idp0;
pub mod mdl5;
pub mod map4;
pub mod packed;
pub mod rada;
pub mod research;
pub mod rt2;
pub mod store;
pub mod txs5;
pub mod udp;

pub use store::Store;
