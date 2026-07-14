use async_trait::async_trait;
use sqlx::{types::Json, PgPool};
use std::collections::BTreeMap;
use ugaris_core::ids::CharacterId;

mod pg;
mod repository;
mod rows;

pub use pg::*;
pub use repository::*;
pub use rows::*;

pub fn legacy_result_name(result: i32) -> &'static str {
    match result {
        0 => "pass",
        1 => "fail",
        2 => "timeout",
        _ => "pass",
    }
}

pub fn legacy_signature_action_name(action: i32) -> &'static str {
    match action {
        0 => "none",
        1 => "flagged",
        2 => "warned",
        3 => "banned",
        _ => "none",
    }
}

pub fn legacy_risk_name(risk: i32) -> &'static str {
    match risk {
        0 => "low",
        1 => "medium",
        2 => "high",
        3 => "critical",
        _ => "low",
    }
}

#[cfg(test)]
mod tests;
