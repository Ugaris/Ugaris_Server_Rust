use async_trait::async_trait;
use sqlx::{types::Json, PgPool};
use std::collections::BTreeMap;
use ugaris_core::ids::CharacterId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AntiCheatSessionCreate {
    pub login_session_id: Option<i64>,
    pub account_id: Option<i64>,
    pub character_id: Option<CharacterId>,
    pub ip_address: i32,
    pub area_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AntiCheatFingerprint {
    pub mod_major: u8,
    pub mod_minor: u8,
    pub mod_patch: u8,
    pub os_type: u8,
    pub screen_w: u16,
    pub screen_h: u16,
    pub hardware_hash: u32,
    pub code_hash: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AntiCheatCounters {
    pub heartbeat_delta: i32,
    pub state_delta: i32,
    pub challenge_delta: i32,
    pub anomaly_delta: i32,
    pub timeout_delta: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AntiCheatEvent {
    pub session_id: i64,
    pub event_type: String,
    pub severity: i32,
    pub details: Option<String>,
    pub data: BTreeMap<String, String>,
}

#[async_trait]
pub trait AntiCheatRepository: Send + Sync {
    async fn create_session(&self, request: AntiCheatSessionCreate) -> anyhow::Result<i64>;
    async fn set_character(
        &self,
        session_id: i64,
        character_id: CharacterId,
    ) -> anyhow::Result<bool>;
    async fn set_fingerprint(
        &self,
        session_id: i64,
        fingerprint: AntiCheatFingerprint,
    ) -> anyhow::Result<bool>;
    async fn set_status(&self, session_id: i64, status: i32) -> anyhow::Result<bool>;
    async fn update_bot_score(
        &self,
        session_id: i64,
        bot_score: f32,
        is_max: bool,
    ) -> anyhow::Result<bool>;
    async fn increment_counters(
        &self,
        session_id: i64,
        counters: AntiCheatCounters,
    ) -> anyhow::Result<bool>;
    async fn end_session(&self, session_id: i64, final_bot_score: f32) -> anyhow::Result<bool>;
    async fn log_event(&self, event: AntiCheatEvent) -> anyhow::Result<i64>;
    async fn cleanup_old_records(&self, days_to_keep: i32) -> anyhow::Result<u64>;
}

#[derive(Debug, Clone)]
pub struct PgAntiCheatRepository {
    pool: PgPool,
}

impl PgAntiCheatRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AntiCheatRepository for PgAntiCheatRepository {
    async fn create_session(&self, request: AntiCheatSessionCreate) -> anyhow::Result<i64> {
        let (session_id,) = sqlx::query_as::<_, (i64,)>(
            "insert into anticheat_sessions(\
             login_session_id, account_id, character_id, ip_address, area_id) \
             values ($1, $2, $3, $4, $5) returning id",
        )
        .bind(request.login_session_id)
        .bind(request.account_id)
        .bind(request.character_id.map(|id| id.0 as i64))
        .bind(request.ip_address)
        .bind(request.area_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(session_id)
    }

    async fn set_character(
        &self,
        session_id: i64,
        character_id: CharacterId,
    ) -> anyhow::Result<bool> {
        let result = sqlx::query("update anticheat_sessions set character_id = $1 where id = $2")
            .bind(character_id.0 as i64)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_fingerprint(
        &self,
        session_id: i64,
        fingerprint: AntiCheatFingerprint,
    ) -> anyhow::Result<bool> {
        let result = sqlx::query(
            "update anticheat_sessions set \
             mod_major = $1, mod_minor = $2, mod_patch = $3, os_type = $4, \
             screen_w = $5, screen_h = $6, hardware_hash = $7, code_hash = $8 \
             where id = $9",
        )
        .bind(i32::from(fingerprint.mod_major))
        .bind(i32::from(fingerprint.mod_minor))
        .bind(i32::from(fingerprint.mod_patch))
        .bind(i32::from(fingerprint.os_type))
        .bind(i32::from(fingerprint.screen_w))
        .bind(i32::from(fingerprint.screen_h))
        .bind(i64::from(fingerprint.hardware_hash))
        .bind(i64::from(fingerprint.code_hash))
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_status(&self, session_id: i64, status: i32) -> anyhow::Result<bool> {
        let result = sqlx::query("update anticheat_sessions set status = $1 where id = $2")
            .bind(status)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn update_bot_score(
        &self,
        session_id: i64,
        bot_score: f32,
        is_max: bool,
    ) -> anyhow::Result<bool> {
        let sql = if is_max {
            "update anticheat_sessions set bot_score = $1, max_bot_score = greatest(max_bot_score, $1) where id = $2"
        } else {
            "update anticheat_sessions set bot_score = $1 where id = $2"
        };
        let result = sqlx::query(sql)
            .bind(bot_score)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn increment_counters(
        &self,
        session_id: i64,
        counters: AntiCheatCounters,
    ) -> anyhow::Result<bool> {
        let result = sqlx::query(
            "update anticheat_sessions set \
             heartbeat_violations = heartbeat_violations + $1, \
             state_violations = state_violations + $2, \
             challenge_failures = challenge_failures + $3, \
             anomaly_count = anomaly_count + $4, timeout_count = timeout_count + $5 \
             where id = $6",
        )
        .bind(counters.heartbeat_delta)
        .bind(counters.state_delta)
        .bind(counters.challenge_delta)
        .bind(counters.anomaly_delta)
        .bind(counters.timeout_delta)
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn end_session(&self, session_id: i64, final_bot_score: f32) -> anyhow::Result<bool> {
        let result = sqlx::query(
            "update anticheat_sessions set ended_at = now(), bot_score = $1 where id = $2",
        )
        .bind(final_bot_score)
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn log_event(&self, event: AntiCheatEvent) -> anyhow::Result<i64> {
        let (event_id,) = sqlx::query_as::<_, (i64,)>(
            "insert into anticheat_events(session_id, event_type, severity, details, data) \
             values ($1, $2, $3, $4, $5) returning id",
        )
        .bind(event.session_id)
        .bind(event.event_type)
        .bind(event.severity)
        .bind(event.details)
        .bind(Json(event.data))
        .fetch_one(&self.pool)
        .await?;

        Ok(event_id)
    }

    async fn cleanup_old_records(&self, days_to_keep: i32) -> anyhow::Result<u64> {
        let result = sqlx::query(
            "delete from anticheat_sessions \
             where ended_at is not null and ended_at < now() - ($1 * interval '1 day')",
        )
        .bind(days_to_keep)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}

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
mod tests {
    use super::*;

    #[test]
    fn legacy_result_names_match_c_defaults() {
        assert_eq!(legacy_result_name(0), "pass");
        assert_eq!(legacy_result_name(1), "fail");
        assert_eq!(legacy_result_name(2), "timeout");
        assert_eq!(legacy_result_name(99), "pass");
    }

    #[test]
    fn legacy_signature_action_names_match_c_defaults() {
        assert_eq!(legacy_signature_action_name(0), "none");
        assert_eq!(legacy_signature_action_name(1), "flagged");
        assert_eq!(legacy_signature_action_name(2), "warned");
        assert_eq!(legacy_signature_action_name(3), "banned");
        assert_eq!(legacy_signature_action_name(99), "none");
    }

    #[test]
    fn legacy_risk_names_match_c_defaults() {
        assert_eq!(legacy_risk_name(0), "low");
        assert_eq!(legacy_risk_name(1), "medium");
        assert_eq!(legacy_risk_name(2), "high");
        assert_eq!(legacy_risk_name(3), "critical");
        assert_eq!(legacy_risk_name(99), "low");
    }
}
