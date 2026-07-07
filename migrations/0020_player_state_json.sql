-- Typed JSON persistence for per-player state (quest PPDs, keyring,
-- achievements, settings, account depot, ...). Replaces the legacy binary
-- PPD/subscriber blobs as the authoritative store; the blob columns remain
-- readable as a migration fallback for rows saved before this column
-- existed. Section names inside the document are the serde field names of
-- `ugaris_core::player::PlayerRuntime`, which makes the state directly
-- queryable for website/launcher/bot integration, e.g.:
--   select player_state_json->'player'->'keyring' from characters ...
alter table characters
    add column if not exists player_state_json jsonb;
