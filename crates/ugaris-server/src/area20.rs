//! Area 20 (Live Quest) server-side glue needing `ZoneLoader`/
//! `PlayerRuntime` - see `ugaris_core::world::npc::area20::lqnpc` for the
//! pure per-tick driver this applies events from.

use super::*;
use ugaris_core::world::{
    make_lq_item_template_id, LqItemSpec, LqNpcOutcomeEvent, LqQuestFile, LqQuestFileDispatch,
};

/// C `create_lq_item` (`src/area/20/lq.c:199-217`): instantiate the
/// `"lq_<base>"` template, apply the admin-authored name/description
/// override, and stamp the `MAKE_ITEMID(DEV_ID_LQ, keyID)` identity plus
/// `IF_LABITEM` - the only place in this port that actually creates one of
/// these items (both `spawn_lq_npc_character`'s `carry_item` and
/// `apply_lqnpc_events`'s `GiveRewardItem` call through here).
pub(crate) fn create_lq_item(
    loader: &mut ZoneLoader,
    world: &mut World,
    owner: Option<CharacterId>,
    spec: &LqItemSpec,
) -> Option<ItemId> {
    let template = format!("lq_{}", spec.base);
    let mut item = loader.instantiate_item_template(&template, owner).ok()?;
    if !spec.name.is_empty() {
        item.name = spec.name.clone();
    }
    if !spec.description.is_empty() {
        item.description = spec.description.clone();
    }
    item.template_id = make_lq_item_template_id(spec.key_id);
    item.flags.insert(ItemFlags::LABITEM);
    let item_id = item.id;
    world.items.insert(item_id, item);
    Some(item_id)
}

/// Applies [`LqNpcOutcomeEvent`]s from `World::process_lqnpc_actions`:
/// player quest-mark writes (`PlayerRuntime::set_lq_mark`) and quest-item
/// turn-in rewards (`create_lq_item` + `World::give_char_item`). Returns
/// the number of events applied.
pub(crate) fn apply_lqnpc_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<LqNpcOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            LqNpcOutcomeEvent::SetPlayerMark { player_id, mark_id } => {
                if let Some(player) = runtime.player_for_character_mut(player_id) {
                    player.set_lq_mark(mark_id);
                    applied += 1;
                }
            }
            LqNpcOutcomeEvent::GiveRewardItem { receiver_id, item } => {
                if let Some(item_id) = create_lq_item(loader, world, Some(receiver_id), &item) {
                    if !world.give_char_item(receiver_id, item_id) {
                        world.destroy_item(item_id);
                    }
                    applied += 1;
                }
            }
        }
    }
    applied
}

/// C's `quest/` save directory (`sprintf(file, "quest/%s.qst", name)`,
/// `lq.c:1371`/`1422`/`1469`), relative to the server's working directory.
pub(crate) const LQ_QUEST_DIR: &str = "quest";

/// C `sprintf(file, "quest/%s.qst", name)` - `name` is already validated
/// `isalpha`-only by `World::try_dispatch_lq_quest_file`, so no
/// path-traversal characters (`.`/`/`) can ever reach here.
fn lq_quest_file_path(quest_dir: &Path, name: &str) -> PathBuf {
    quest_dir.join(format!("{name}.qst"))
}

/// The stored password matches C's own gate
/// (`if (xpassword[0] && strcmp(xpassword, password))`, `lq.c:1375`/
/// `1433`/`1487`): an empty stored password never blocks anything, a
/// non-empty one must match exactly.
fn lq_quest_password_matches(stored: &str, given: &str) -> bool {
    stored.is_empty() || stored == given
}

/// Executes a [`LqQuestFileDispatch::Save`]/`Delete`/`Load` outcome
/// against `<quest_dir>/<name>.qst` - the actual filesystem half `World`
/// can't perform itself (see `ugaris_core::world::lq_quest_file`'s module
/// doc comment). All caller feedback is queued directly onto `world`'s
/// existing pending-text queues, exactly like every other `CDR_LQPARSER`
/// command in this table. `quest_dir` is [`LQ_QUEST_DIR`] at the real
/// call site; parameterized so tests can point at a scratch directory
/// instead of touching the server's real working directory.
pub(crate) fn handle_lq_quest_file_dispatch(
    world: &mut World,
    character_id: CharacterId,
    dispatch: LqQuestFileDispatch,
    quest_dir: &Path,
) {
    match dispatch {
        LqQuestFileDispatch::NotMatched | LqQuestFileDispatch::Rejected => {}
        LqQuestFileDispatch::Save { name, password } => {
            let path = lq_quest_file_path(quest_dir, &name);
            if let Ok(existing) = std::fs::read_to_string(&path) {
                let stored_password = serde_json::from_str::<LqQuestFile>(&existing)
                    .map(|file| file.password)
                    .unwrap_or_default();
                if !lq_quest_password_matches(&stored_password, &password) {
                    world.queue_lq_error(
                        character_id,
                        "Cannot overwrite file with differing password.",
                    );
                    return;
                }
            }
            let file = LqQuestFile {
                password: password.clone(),
                snapshot: world.lq_quest_snapshot(),
            };
            let written = std::fs::create_dir_all(quest_dir)
                .ok()
                .and_then(|()| serde_json::to_string_pretty(&file).ok())
                .and_then(|json| std::fs::write(&path, json).ok());
            if written.is_some() {
                world.queue_system_text(
                    character_id,
                    format!("Saved as {name}, password \"{password}\"."),
                );
            } else {
                world.queue_lq_error(character_id, "Cannot create file.");
            }
        }
        LqQuestFileDispatch::Delete { name, password } => {
            let path = lq_quest_file_path(quest_dir, &name);
            let Ok(existing) = std::fs::read_to_string(&path) else {
                world.queue_lq_error(character_id, "File not found.");
                return;
            };
            let stored_password = serde_json::from_str::<LqQuestFile>(&existing)
                .map(|file| file.password)
                .unwrap_or_default();
            if !lq_quest_password_matches(&stored_password, &password) {
                world.queue_lq_error(character_id, "Cannot delete file with differing password.");
                return;
            }
            if std::fs::remove_file(&path).is_ok() {
                world.queue_system_text(character_id, format!("Deleted quest {name}."));
            } else {
                world.queue_lq_error(character_id, "Delete failed at system level.");
            }
        }
        LqQuestFileDispatch::Load { name, password } => {
            let path = lq_quest_file_path(quest_dir, &name);
            let Ok(existing) = std::fs::read_to_string(&path) else {
                world.queue_lq_error(character_id, "File not found.");
                return;
            };
            let Ok(file) = serde_json::from_str::<LqQuestFile>(&existing) else {
                world.queue_lq_error(
                    character_id,
                    "The file appears to be a different version. Cannot load.",
                );
                return;
            };
            if !lq_quest_password_matches(&file.password, &password) {
                world.queue_lq_error(character_id, "Cannot load file with differing password.");
                return;
            }
            world.apply_lq_quest_snapshot(file.snapshot);
            world.queue_system_text(character_id, format!("Loaded quest {name}."));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ugaris_lq_quest_file_test_{label}_{}_{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn caller(world: &mut World) -> CharacterId {
        let character_id = CharacterId(1);
        let login = LoginBlock {
            name: "Godmode".into(),
            password: String::new(),
            vendor: 0,
            client_version: Some(3),
            his_ip: 0,
            our_ip: 0,
            unique: 0,
        };
        let mut character = crate::login::login_character(character_id, &login, 20, 10, 10);
        character.flags |= CharacterFlags::GOD;
        world.characters.insert(character_id, character);
        character_id
    }

    fn plain_texts(world: &mut World) -> Vec<String> {
        world
            .drain_pending_system_texts()
            .into_iter()
            .map(|event| event.message)
            .collect()
    }

    fn error_texts(world: &mut World) -> Vec<String> {
        world
            .drain_pending_system_text_bytes()
            .into_iter()
            .map(|event| {
                String::from_utf8_lossy(&event.message[COL_LIGHT_RED.len()..]).into_owned()
            })
            .collect()
    }

    #[test]
    fn save_then_load_round_trips_lq_data() {
        let dir = scratch_dir("save_load");
        let mut world = World::default();
        let character_id = caller(&mut world);
        world.lq_data.min_level = 3;
        world.lq_data.max_level = 40;

        let dispatch =
            world.try_dispatch_lq_quest_file(character_id, 20, "#questsave roundtrip secret");
        handle_lq_quest_file_dispatch(&mut world, character_id, dispatch, &dir);
        assert_eq!(
            plain_texts(&mut world),
            vec!["Saved as roundtrip, password \"secret\".".to_string()]
        );

        // Mutate world state, then load it back from the file.
        world.lq_data.min_level = 0;
        let dispatch =
            world.try_dispatch_lq_quest_file(character_id, 20, "#questload roundtrip secret");
        handle_lq_quest_file_dispatch(&mut world, character_id, dispatch, &dir);
        assert_eq!(
            plain_texts(&mut world),
            vec!["Loaded quest roundtrip.".to_string()]
        );
        assert_eq!(world.lq_data.min_level, 3);
        assert_eq!(world.lq_data.max_level, 40);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_reports_file_not_found() {
        let dir = scratch_dir("missing");
        let mut world = World::default();
        let character_id = caller(&mut world);
        let dispatch = world.try_dispatch_lq_quest_file(character_id, 20, "#questload nosuchfile");
        handle_lq_quest_file_dispatch(&mut world, character_id, dispatch, &dir);
        assert_eq!(error_texts(&mut world), vec!["File not found.".to_string()]);
    }

    #[test]
    fn save_rejects_mismatched_password_on_overwrite() {
        let dir = scratch_dir("save_pw");
        let mut world = World::default();
        let character_id = caller(&mut world);

        let dispatch =
            world.try_dispatch_lq_quest_file(character_id, 20, "#questsave locked correct");
        handle_lq_quest_file_dispatch(&mut world, character_id, dispatch, &dir);
        plain_texts(&mut world);

        let dispatch =
            world.try_dispatch_lq_quest_file(character_id, 20, "#questsave locked wrong");
        handle_lq_quest_file_dispatch(&mut world, character_id, dispatch, &dir);
        assert_eq!(
            error_texts(&mut world),
            vec!["Cannot overwrite file with differing password.".to_string()]
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_rejects_mismatched_password() {
        let dir = scratch_dir("load_pw");
        let mut world = World::default();
        let character_id = caller(&mut world);

        let dispatch =
            world.try_dispatch_lq_quest_file(character_id, 20, "#questsave locked correct");
        handle_lq_quest_file_dispatch(&mut world, character_id, dispatch, &dir);
        plain_texts(&mut world);

        let dispatch =
            world.try_dispatch_lq_quest_file(character_id, 20, "#questload locked wrong");
        handle_lq_quest_file_dispatch(&mut world, character_id, dispatch, &dir);
        assert_eq!(
            error_texts(&mut world),
            vec!["Cannot load file with differing password.".to_string()]
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_removes_the_file_and_a_second_delete_reports_not_found() {
        let dir = scratch_dir("delete");
        let mut world = World::default();
        let character_id = caller(&mut world);

        let dispatch = world.try_dispatch_lq_quest_file(character_id, 20, "#questsave gone");
        handle_lq_quest_file_dispatch(&mut world, character_id, dispatch, &dir);
        plain_texts(&mut world);

        let dispatch = world.try_dispatch_lq_quest_file(character_id, 20, "#questdelete gone");
        handle_lq_quest_file_dispatch(&mut world, character_id, dispatch, &dir);
        assert_eq!(
            plain_texts(&mut world),
            vec!["Deleted quest gone.".to_string()]
        );

        let dispatch = world.try_dispatch_lq_quest_file(character_id, 20, "#questdelete gone");
        handle_lq_quest_file_dispatch(&mut world, character_id, dispatch, &dir);
        assert_eq!(error_texts(&mut world), vec!["File not found.".to_string()]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_rejects_mismatched_password() {
        let dir = scratch_dir("delete_pw");
        let mut world = World::default();
        let character_id = caller(&mut world);

        let dispatch =
            world.try_dispatch_lq_quest_file(character_id, 20, "#questsave locked correct");
        handle_lq_quest_file_dispatch(&mut world, character_id, dispatch, &dir);
        plain_texts(&mut world);

        let dispatch =
            world.try_dispatch_lq_quest_file(character_id, 20, "#questdelete locked wrong");
        handle_lq_quest_file_dispatch(&mut world, character_id, dispatch, &dir);
        assert_eq!(
            error_texts(&mut world),
            vec!["Cannot delete file with differing password.".to_string()]
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_of_corrupt_file_reports_version_mismatch() {
        let dir = scratch_dir("corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("bad.qst"), b"not json").unwrap();
        let mut world = World::default();
        let character_id = caller(&mut world);

        let dispatch = world.try_dispatch_lq_quest_file(character_id, 20, "#questload bad");
        handle_lq_quest_file_dispatch(&mut world, character_id, dispatch, &dir);
        assert_eq!(
            error_texts(&mut world),
            vec!["The file appears to be a different version. Cannot load.".to_string()]
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
