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
pub mod garage;
pub mod gpb;
pub mod gpb4;
pub mod idp0;
pub mod jxl;
pub mod map4;
pub mod mdl5;
pub mod menu;
pub mod official;
pub mod packed;
pub mod rada;
pub mod research;
pub mod rt2;
pub mod site;
pub mod sndz;
pub mod store;
pub mod tools;
pub mod txs5;
pub mod udp;
pub mod verify;
pub mod vehicle_i18n;

pub use store::Store;
