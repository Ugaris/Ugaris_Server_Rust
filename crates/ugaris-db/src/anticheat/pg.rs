use super::*;

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

    async fn reset_session(&self, session_id: i64) -> anyhow::Result<bool> {
        let result = sqlx::query(
            "update anticheat_sessions set \
             status = 1, bot_score = 0, heartbeat_violations = 0, \
             state_violations = 0, challenge_failures = 0, timeout_count = 0 \
             where id = $1",
        )
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn find_session(&self, session_id: i64) -> anyhow::Result<Option<AntiCheatSessionInfo>> {
        let row = sqlx::query_as::<
            _,
            (
                i32,
                f32,
                i32,
                i32,
                i32,
                i32,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
            ),
        >(
            "select status, bot_score, heartbeat_violations, state_violations, \
             challenge_failures, timeout_count, mod_major, mod_minor, mod_patch, \
             os_type, screen_w, screen_h \
             from anticheat_sessions where id = $1",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(
                status,
                bot_score,
                heartbeat_violations,
                state_violations,
                challenge_failures,
                timeout_count,
                mod_major,
                mod_minor,
                mod_patch,
                os_type,
                screen_w,
                screen_h,
            )| AntiCheatSessionInfo {
                status,
                bot_score,
                heartbeat_violations,
                state_violations,
                challenge_failures,
                timeout_count,
                mod_major,
                mod_minor,
                mod_patch,
                os_type,
                screen_w,
                screen_h,
            },
        ))
    }

    async fn find_sessions(
        &self,
        session_ids: &[i64],
    ) -> anyhow::Result<Vec<(i64, AntiCheatSessionInfo)>> {
        if session_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<
            _,
            (
                i64,
                i32,
                f32,
                i32,
                i32,
                i32,
                i32,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
            ),
        >(
            "select id, status, bot_score, heartbeat_violations, state_violations, \
             challenge_failures, timeout_count, mod_major, mod_minor, mod_patch, \
             os_type, screen_w, screen_h \
             from anticheat_sessions where id = any($1)",
        )
        .bind(session_ids)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    status,
                    bot_score,
                    heartbeat_violations,
                    state_violations,
                    challenge_failures,
                    timeout_count,
                    mod_major,
                    mod_minor,
                    mod_patch,
                    os_type,
                    screen_w,
                    screen_h,
                )| {
                    (
                        id,
                        AntiCheatSessionInfo {
                            status,
                            bot_score,
                            heartbeat_violations,
                            state_violations,
                            challenge_failures,
                            timeout_count,
                            mod_major,
                            mod_minor,
                            mod_patch,
                            os_type,
                            screen_w,
                            screen_h,
                        },
                    )
                },
            )
            .collect())
    }

    async fn account_id_for_session(&self, session_id: i64) -> anyhow::Result<Option<i64>> {
        let row = sqlx::query_as::<_, (Option<i64>,)>(
            "select account_id from anticheat_sessions where id = $1",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|(account_id,)| account_id))
    }

    async fn set_flagged(&self, subscriber_id: i64, is_flagged: bool) -> anyhow::Result<()> {
        sqlx::query(
            "insert into ac_player_stats (subscriber_id, is_flagged, updated_at) \
             values ($1, $2, now()) \
             on conflict (subscriber_id) do update set is_flagged = $2, updated_at = now()",
        )
        .bind(subscriber_id)
        .bind(is_flagged)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn set_trusted(&self, subscriber_id: i64, is_trusted: bool) -> anyhow::Result<()> {
        sqlx::query(
            "insert into ac_player_stats (subscriber_id, is_trusted, updated_at) \
             values ($1, $2, now()) \
             on conflict (subscriber_id) do update set is_trusted = $2, updated_at = now()",
        )
        .bind(subscriber_id)
        .bind(is_trusted)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn issue_warning(&self, subscriber_id: i64) -> anyhow::Result<()> {
        sqlx::query(
            "insert into ac_player_stats (subscriber_id, warnings_issued, last_warning_at, \
             updated_at) values ($1, 1, now(), now()) \
             on conflict (subscriber_id) do update set \
             warnings_issued = ac_player_stats.warnings_issued + 1, last_warning_at = now(), \
             updated_at = now()",
        )
        .bind(subscriber_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn recent_sessions(
        &self,
        account_id: i64,
        max_count: i64,
    ) -> anyhow::Result<Vec<AntiCheatSessionHistoryRow>> {
        let rows = sqlx::query_as::<_, (String, i32, i32, f32, i32, i32, i32, i32)>(
            "select to_char(started_at, 'MM-DD HH24:MI'), \
             (extract(epoch from (coalesce(ended_at, now()) - started_at)) / 60)::int, \
             status, bot_score, heartbeat_violations, state_violations, \
             challenge_failures, anomaly_count \
             from anticheat_sessions where account_id = $1 \
             order by started_at desc limit $2",
        )
        .bind(account_id)
        .bind(max_count)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    start_time,
                    duration_minutes,
                    status,
                    bot_score,
                    heartbeat_violations,
                    state_violations,
                    challenge_failures,
                    anomaly_count,
                )| AntiCheatSessionHistoryRow {
                    start_time,
                    duration_minutes,
                    status,
                    bot_score,
                    heartbeat_violations,
                    state_violations,
                    challenge_failures,
                    anomaly_count,
                },
            )
            .collect())
    }

    async fn recent_violations(
        &self,
        account_id: i64,
        max_count: i64,
    ) -> anyhow::Result<Vec<AntiCheatViolationRow>> {
        let rows = sqlx::query_as::<_, (String, String, i32, Option<String>)>(
            "select to_char(e.created_at, 'MM-DD HH24:MI'), e.event_type, e.severity, e.details \
             from anticheat_events e \
             join anticheat_sessions s on s.id = e.session_id \
             where s.account_id = $1 \
             order by e.created_at desc limit $2",
        )
        .bind(account_id)
        .bind(max_count)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(detected_at, type_name, severity, details)| AntiCheatViolationRow {
                    detected_at,
                    type_name,
                    severity,
                    details,
                },
            )
            .collect())
    }

    async fn list_signatures(&self, max_count: i64) -> anyhow::Result<Vec<AntiCheatSignatureRow>> {
        let rows = sqlx::query_as::<_, (i64, String, String, i32, bool, bool, i32, bool)>(
            "select id, signature_type, name, severity, auto_flag, auto_ban, times_detected, \
             is_active from ac_known_signatures order by times_detected desc limit $1",
        )
        .bind(max_count)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    signature_type,
                    name,
                    severity,
                    auto_flag,
                    auto_ban,
                    times_detected,
                    is_active,
                )| {
                    AntiCheatSignatureRow {
                        id,
                        signature_type,
                        name,
                        severity,
                        auto_flag,
                        auto_ban,
                        times_detected,
                        is_active,
                    }
                },
            )
            .collect())
    }

    async fn add_signature(
        &self,
        signature_type: &str,
        signature_value: &str,
        name: &str,
        created_by: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "insert into ac_known_signatures (signature_type, signature_value, name, created_by) \
             values ($1, $2, $3, $4)",
        )
        .bind(signature_type)
        .bind(signature_value)
        .bind(name)
        .bind(created_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_signature(&self, signature_id: i64) -> anyhow::Result<bool> {
        let result = sqlx::query("delete from ac_known_signatures where id = $1")
            .bind(signature_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn update_player_stats(
        &self,
        subscriber_id: i64,
        session_bot_score: f32,
        session_status: i32,
        heartbeat_violations: i32,
        state_violations: i32,
        challenge_failures: i32,
        anomalies: i32,
    ) -> anyhow::Result<()> {
        let was_flagged: i32 = if session_status == 3 { 1 } else { 0 };
        let was_suspicious: i32 = if session_status == 2 { 1 } else { 0 };
        sqlx::query(
            "insert into ac_player_stats (\
             subscriber_id, total_sessions, flagged_sessions, suspicious_sessions, \
             total_heartbeat_violations, total_state_violations, total_challenge_failures, \
             total_anomalies, lifetime_bot_score, max_session_bot_score, \
             avg_session_bot_score, risk_level, last_seen, updated_at) \
             values ($1, 1, $3, $4, $5, $6, $7, $8, $2, $2, $2, \
             case \
                 when $2 >= 1.0 or $3 >= 3 then 'critical' \
                 when $2 >= 0.8 or $3 >= 1 then 'high' \
                 when $2 >= 0.5 or $4 >= 3 then 'medium' \
                 else 'low' \
             end, \
             now(), now()) \
             on conflict (subscriber_id) do update set \
             total_sessions = ac_player_stats.total_sessions + 1, \
             flagged_sessions = ac_player_stats.flagged_sessions + excluded.flagged_sessions, \
             suspicious_sessions = \
                 ac_player_stats.suspicious_sessions + excluded.suspicious_sessions, \
             total_heartbeat_violations = \
                 ac_player_stats.total_heartbeat_violations + excluded.total_heartbeat_violations, \
             total_state_violations = \
                 ac_player_stats.total_state_violations + excluded.total_state_violations, \
             total_challenge_failures = \
                 ac_player_stats.total_challenge_failures + excluded.total_challenge_failures, \
             total_anomalies = ac_player_stats.total_anomalies + excluded.total_anomalies, \
             lifetime_bot_score = ac_player_stats.lifetime_bot_score + excluded.lifetime_bot_score, \
             max_session_bot_score = \
                 greatest(ac_player_stats.max_session_bot_score, excluded.max_session_bot_score), \
             avg_session_bot_score = \
                 (ac_player_stats.lifetime_bot_score + excluded.lifetime_bot_score) \
                 / (ac_player_stats.total_sessions + 1), \
             risk_level = case \
                 when greatest(ac_player_stats.max_session_bot_score, \
                               excluded.max_session_bot_score) >= 1.0 \
                      or (ac_player_stats.flagged_sessions + excluded.flagged_sessions) >= 3 \
                 then 'critical' \
                 when greatest(ac_player_stats.max_session_bot_score, \
                               excluded.max_session_bot_score) >= 0.8 \
                      or (ac_player_stats.flagged_sessions + excluded.flagged_sessions) >= 1 \
                 then 'high' \
                 when greatest(ac_player_stats.max_session_bot_score, \
                               excluded.max_session_bot_score) >= 0.5 \
                      or (ac_player_stats.suspicious_sessions + excluded.suspicious_sessions) >= 3 \
                 then 'medium' \
                 else 'low' \
             end, \
             last_seen = now(), updated_at = now()",
        )
        .bind(subscriber_id)
        .bind(session_bot_score)
        .bind(was_flagged)
        .bind(was_suspicious)
        .bind(heartbeat_violations)
        .bind(state_violations)
        .bind(challenge_failures)
        .bind(anomalies)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_player_stats(
        &self,
        subscriber_id: i64,
    ) -> anyhow::Result<Option<AntiCheatPlayerStatsRow>> {
        let row = sqlx::query_as::<
            _,
            (
                i32,
                i32,
                i32,
                i32,
                i32,
                i32,
                i32,
                f32,
                f32,
                String,
                bool,
                bool,
                i32,
                String,
                Option<String>,
            ),
        >(
            "select total_sessions, flagged_sessions, suspicious_sessions, \
             total_heartbeat_violations, total_state_violations, total_challenge_failures, \
             total_anomalies, max_session_bot_score, avg_session_bot_score, risk_level, \
             is_flagged, is_trusted, warnings_issued, to_char(first_seen, 'MM-DD HH24:MI'), \
             to_char(last_seen, 'MM-DD HH24:MI') \
             from ac_player_stats where subscriber_id = $1",
        )
        .bind(subscriber_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(
                total_sessions,
                flagged_sessions,
                suspicious_sessions,
                total_heartbeat_violations,
                total_state_violations,
                total_challenge_failures,
                total_anomalies,
                max_session_bot_score,
                avg_session_bot_score,
                risk_level,
                is_flagged,
                is_trusted,
                warnings_issued,
                first_seen,
                last_seen,
            )| AntiCheatPlayerStatsRow {
                total_sessions,
                flagged_sessions,
                suspicious_sessions,
                total_heartbeat_violations,
                total_state_violations,
                total_challenge_failures,
                total_anomalies,
                max_session_bot_score,
                avg_session_bot_score,
                risk_level,
                is_flagged,
                is_trusted,
                warnings_issued,
                first_seen,
                last_seen,
            },
        ))
    }

    async fn shared_ips(
        &self,
        account_id: i64,
        max_count: i64,
    ) -> anyhow::Result<Vec<AntiCheatSharedIpRow>> {
        let rows = sqlx::query_as::<_, (String, i32, i64, String)>(
            "select a2.username, s2.ip_address, count(*) as session_count, \
             to_char(max(s2.started_at), 'YYYY-MM-DD') as last_seen \
             from anticheat_sessions s1 \
             join anticheat_sessions s2 \
               on s2.ip_address = s1.ip_address and s2.account_id != s1.account_id \
             join accounts a2 on a2.id = s2.account_id \
             where s1.account_id = $1 \
             group by a2.username, s2.ip_address \
             order by max(s2.started_at) desc \
             limit $2",
        )
        .bind(account_id)
        .bind(max_count)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(username, ip_address, session_count, last_seen)| AntiCheatSharedIpRow {
                    username,
                    ip_address,
                    session_count,
                    last_seen,
                },
            )
            .collect())
    }

    async fn shared_hardware(
        &self,
        account_id: i64,
        max_count: i64,
    ) -> anyhow::Result<Vec<AntiCheatSharedHwRow>> {
        let rows = sqlx::query_as::<_, (String, i64, Option<i32>, Option<i32>, String)>(
            "select a2.username, s2.hardware_hash, s2.screen_w, s2.screen_h, \
             to_char(max(s2.started_at), 'YYYY-MM-DD') as last_seen \
             from anticheat_sessions s1 \
             join anticheat_sessions s2 \
               on s2.hardware_hash = s1.hardware_hash and s2.account_id != s1.account_id \
             join accounts a2 on a2.id = s2.account_id \
             where s1.account_id = $1 and s1.hardware_hash is not null \
             group by a2.username, s2.hardware_hash, s2.screen_w, s2.screen_h \
             order by max(s2.started_at) desc \
             limit $2",
        )
        .bind(account_id)
        .bind(max_count)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(username, hardware_hash, screen_w, screen_h, last_seen)| AntiCheatSharedHwRow {
                    username,
                    hardware_hash,
                    screen_w,
                    screen_h,
                    last_seen,
                },
            )
            .collect())
    }

    async fn high_risk_players(&self, max_count: i64) -> anyhow::Result<Vec<AntiCheatHighRiskRow>> {
        let rows = sqlx::query_as::<_, (i64, String, String, f32, i32, Option<String>)>(
            "select ps.subscriber_id, a.username, ps.risk_level, ps.max_session_bot_score, \
             ps.flagged_sessions, to_char(ps.last_seen, 'MM-DD HH24:MI') \
             from ac_player_stats ps \
             join accounts a on a.id = ps.subscriber_id \
             where ps.risk_level in ('high', 'critical') or ps.is_flagged = true \
             order by ps.max_session_bot_score desc \
             limit $1",
        )
        .bind(max_count)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    subscriber_id,
                    username,
                    risk_level,
                    max_bot_score,
                    flagged_sessions,
                    last_seen,
                )| {
                    AntiCheatHighRiskRow {
                        subscriber_id,
                        username,
                        risk_level,
                        max_bot_score,
                        flagged_sessions,
                        last_seen,
                    }
                },
            )
            .collect())
    }

    async fn lookup_subscriber(
        &self,
        subscriber_id: i64,
    ) -> anyhow::Result<Option<AntiCheatSubscriberLookup>> {
        let account = sqlx::query_as::<_, (String,)>("select username from accounts where id = $1")
            .bind(subscriber_id)
            .fetch_optional(&self.pool)
            .await?;
        let Some((username,)) = account else {
            return Ok(None);
        };
        let stats = self.find_player_stats(subscriber_id).await?;
        Ok(Some(AntiCheatSubscriberLookup { username, stats }))
    }
}
