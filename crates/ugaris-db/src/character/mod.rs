use async_trait::async_trait;
use sqlx::{postgres::PgArguments, query::Query, types::Json, PgPool, Postgres, Transaction};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use ugaris_core::{
    entity::{Character, CharacterFlags, Item, INVENTORY_SIZE},
    ids::{CharacterId, ItemId},
};

mod pg;
mod types;

pub use pg::*;
pub use types::*;

#[cfg(test)]
mod tests;
