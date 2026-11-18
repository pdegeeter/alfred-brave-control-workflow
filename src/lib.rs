//! Core library for the `alfred-brave` Alfred workflow.
//!
//! The binary is a thin dispatcher over these modules. Everything that can be
//! made pure (parsing, filtering, SQL building, item building) lives here and
//! is unit tested; the only impure boundary is [`runner::CommandRunner`], which
//! is mocked in tests.

pub mod action;
pub mod alfred;
pub mod brave;
pub mod history;
pub mod markdown;
pub mod profiles;
pub mod query;
pub mod runner;
pub mod tabs;
