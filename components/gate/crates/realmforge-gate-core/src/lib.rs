// SPDX-License-Identifier: AGPL-3.0-only

//! Core domain types, errors, configuration, and crypto helpers.
//!
//! This crate is pure: no async runtime, no database, no network. It is the
//! shared foundation imported by `realmforge-gate-db`, `realmforge-gate-oauth`, and
//! `realmforge-gate-account`.
//!
//! Milestone 1 adds configuration, errors, and domain entities. SRP6a lands
//! in Milestone 4 (`srp`); JWT/JWKS signing lands in Milestone 5 (`jwt`).

pub mod config;
pub mod domain;
pub mod encrypted_ticket;
pub mod error;
pub mod jwt;
pub mod key_meta;
pub mod srp;

pub use config::Config;
pub use domain::{
    Account, AuthorizationCode, Credential, OAuthClient, RefreshToken, ServiceTicket, Session,
};
pub use error::Error;
