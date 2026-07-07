use super::*;

/// One-off startup routine for the "Retire legacy blob writes"
/// `PORTING_TODO.md` task. `snapshots.rs` no longer writes `ppd_blob`/
/// `subscriber_blob` (frozen at whatever value they held before the
/// retirement), so every row saved since migration 0020 already has a
/// `player_state_json` document; this backfills the remaining pre-0020
/// rows once, at startup, so a login/backup save for one of them never
/// needs to fall back to the `#[deprecated]` legacy decoders again.
///
/// Mirrors `snapshots.rs::apply_character_snapshot`'s pre-0020 fallback
/// branch exactly (same decoders, same block priority), just against a
/// freshly constructed `PlayerRuntime` instead of a live login session -
/// only the decoded persistent fields matter here, never anything
/// connection/session-shaped, so `PlayerRuntime::connected(0, 0)` is a
/// safe stand-in starting point.
///
/// Returns the number of rows successfully backfilled. A row that fails
/// to decode or serialize is logged and left for the next startup to
/// retry (it still has `player_state_json is null`, so it's picked up
/// again next time) rather than aborting the whole scan.
pub(crate) async fn backfill_legacy_player_state(
    characters: &impl CharacterRepository,
) -> anyhow::Result<u64> {
    let rows = characters.find_legacy_blob_only_characters().await?;
    let mut backfilled = 0u64;

    for LegacyBlobRow {
        character_id,
        ppd_blob,
        subscriber_blob,
    } in rows
    {
        let mut player = PlayerRuntime::connected(0, 0);

        #[allow(deprecated)]
        let (account_depot, ppd_decode_ok) = {
            let account_depot = decode_legacy_account_depot_subscriber_blob(&subscriber_blob);
            if let Some(data) = decode_legacy_achievement_data_subscriber_blob(&subscriber_blob) {
                player.achievement_data = data;
            }
            if let Some(stats) = decode_legacy_achievement_stats_subscriber_blob(&subscriber_blob) {
                player.achievement_stats = stats;
            }
            let ppd_decode_ok = ppd_blob.is_empty() || player.decode_legacy_ppd_blob(&ppd_blob);
            (account_depot, ppd_decode_ok)
        };
        if !ppd_decode_ok {
            warn!(
                character_id = character_id.0,
                "legacy backfill: failed to decode legacy PPD blob; row left for next startup"
            );
            continue;
        }

        let Some(player_state_json) = persisted_player_state_json(&player, account_depot.as_ref())
        else {
            warn!(
                character_id = character_id.0,
                "legacy backfill: failed to serialize decoded player state; row left for next startup"
            );
            continue;
        };

        if let Err(err) = characters
            .backfill_player_state_json(character_id, player_state_json)
            .await
        {
            warn!(
                character_id = character_id.0,
                error = %err,
                "legacy backfill: failed to write player_state_json"
            );
            continue;
        }
        backfilled += 1;
    }

    Ok(backfilled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// In-memory `CharacterRepository` stub covering only the two methods
    /// this module calls, matching every other test double's approach in
    /// this codebase of implementing just enough of a trait to exercise
    /// the code under test (the trait's other methods panic if called,
    /// which would itself be a test failure signal).
    #[derive(Default)]
    struct StubCharacterRepository {
        rows: Vec<LegacyBlobRow>,
        written: Mutex<Vec<(CharacterId, serde_json::Value)>>,
    }

    #[async_trait::async_trait]
    impl CharacterRepository for StubCharacterRepository {
        async fn find_login_target(
            &self,
            _name: &str,
        ) -> anyhow::Result<Option<ugaris_db::character::CharacterSummary>> {
            unimplemented!()
        }
        async fn find_last_seen(
            &self,
            _name: &str,
        ) -> anyhow::Result<Option<ugaris_db::LastSeenInfo>> {
            unimplemented!()
        }
        async fn begin_login(&self, _request: LoginRequest) -> anyhow::Result<LoginOutcome> {
            unimplemented!()
        }
        async fn save_character_snapshot(
            &self,
            _request: CharacterSaveRequest,
        ) -> anyhow::Result<bool> {
            unimplemented!()
        }
        async fn load_character_snapshot(
            &self,
            _character_id: CharacterId,
        ) -> anyhow::Result<Option<CharacterSnapshot>> {
            unimplemented!()
        }
        async fn release_character(&self, _character_id: CharacterId) -> anyhow::Result<()> {
            unimplemented!()
        }
        async fn rename_character(&self, _from: &str, _to: &str) -> anyhow::Result<bool> {
            unimplemented!()
        }
        async fn lock_name(&self, _name: &str) -> anyhow::Result<bool> {
            unimplemented!()
        }
        async fn unlock_name(&self, _name: &str) -> anyhow::Result<bool> {
            unimplemented!()
        }
        async fn set_character_locked(
            &self,
            _character_id: CharacterId,
            _locked: bool,
        ) -> anyhow::Result<()> {
            unimplemented!()
        }
        async fn find_name_by_id(
            &self,
            _character_id: CharacterId,
        ) -> anyhow::Result<Option<String>> {
            unimplemented!()
        }
        async fn find_paid_until_info(
            &self,
            _character_id: CharacterId,
        ) -> anyhow::Result<Option<ugaris_db::character::PaidUntilInfo>> {
            unimplemented!()
        }
        async fn exterminate_account(
            &self,
            _name: &str,
        ) -> anyhow::Result<Option<ugaris_db::character::ExterminateOutcome>> {
            unimplemented!()
        }
        async fn find_legacy_blob_only_characters(&self) -> anyhow::Result<Vec<LegacyBlobRow>> {
            Ok(self.rows.clone())
        }
        async fn backfill_player_state_json(
            &self,
            character_id: CharacterId,
            player_state_json: serde_json::Value,
        ) -> anyhow::Result<()> {
            self.written
                .lock()
                .unwrap()
                .push((character_id, player_state_json));
            Ok(())
        }
    }

    #[tokio::test]
    async fn no_stale_rows_is_a_cheap_no_op() {
        let repo = StubCharacterRepository::default();
        let backfilled = backfill_legacy_player_state(&repo).await.unwrap();
        assert_eq!(backfilled, 0);
        assert!(repo.written.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn empty_blobs_still_backfill_a_minimal_document() {
        // C: a row can reach `player_state_json is null and (ppd_blob <>
        // '' or subscriber_blob <> '')` with one blob empty and the other
        // non-empty (that's the exact `find_legacy_blob_only_characters`
        // predicate); an all-empty-payload row like this one (both blobs
        // present but zero-length after slicing) should still decode to
        // "nothing to restore" and produce a writable, empty-state
        // document rather than being skipped.
        let repo = StubCharacterRepository {
            rows: vec![LegacyBlobRow {
                character_id: CharacterId(7),
                ppd_blob: Vec::new(),
                subscriber_blob: Vec::new(),
            }],
            written: Mutex::new(Vec::new()),
        };
        let backfilled = backfill_legacy_player_state(&repo).await.unwrap();
        assert_eq!(backfilled, 1);
        let written = repo.written.lock().unwrap();
        assert_eq!(written.len(), 1);
        assert_eq!(written[0].0, CharacterId(7));
        assert!(written[0].1.get("player").is_some());
    }

    #[tokio::test]
    async fn corrupt_ppd_blob_is_skipped_and_not_written() {
        let repo = StubCharacterRepository {
            rows: vec![LegacyBlobRow {
                character_id: CharacterId(9),
                ppd_blob: vec![0xff, 0xff, 0xff, 0xff],
                subscriber_blob: Vec::new(),
            }],
            written: Mutex::new(Vec::new()),
        };
        let backfilled = backfill_legacy_player_state(&repo).await.unwrap();
        assert_eq!(backfilled, 0);
        assert!(repo.written.lock().unwrap().is_empty());
    }
}
