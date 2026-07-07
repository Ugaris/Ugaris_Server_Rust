use super::*;

/// Mirrors `ugaris_core::world::combat::RuntimePlayerAttackPolicy`'s shape
/// (see that struct's doc comment) - a separate copy is needed here
/// because these call sites go through `World::tick_effects_with_attack_policy`/
/// `tick_basic_actions_with_attack_policy`'s `FnMut` closures, which cannot
/// hold a live `&World` borrow (the tick call itself needs `&mut World`);
/// callers must clone `world.clan_registry.relations()` before the tick
/// call and move the clone into the closure (see `main.rs`).
pub(crate) struct RuntimePlayerAttackPolicy<'a> {
    pub(crate) attacker_runtime: &'a PlayerRuntime,
    pub(crate) clan_relations: &'a ClanRelations,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PkRelationSnapshot {
    pub(crate) hate_by_character: HashMap<CharacterId, Vec<u32>>,
}

impl PkRelationSnapshot {
    pub(crate) fn from_runtime(runtime: &ServerRuntime) -> Self {
        let hate_by_character = runtime
            .players
            .values()
            .filter_map(|player| {
                let character_id = player.character_id?;
                Some((character_id, player.pk_hate.clone()))
            })
            .collect();
        Self { hate_by_character }
    }

    pub(crate) fn has_hate(&self, source: CharacterId, target: CharacterId) -> bool {
        target.0 != 0
            && self
                .hate_by_character
                .get(&source)
                .is_some_and(|hate| hate.iter().any(|id| *id == target.0))
    }
}

impl ClanAttackPolicy for RuntimePlayerAttackPolicy<'_> {
    fn has_pk_hate(&self, _attacker: &Character, defender: &Character) -> bool {
        self.attacker_runtime.has_pk_hate_for(defender.id.0)
    }

    fn are_allied(&self, attacker_clan: u16, defender_clan: u16) -> bool {
        self.clan_relations.alliance(attacker_clan, defender_clan)
    }

    fn can_attack_inside_clan_area(&self, attacker_clan: u16, defender_clan: u16) -> bool {
        self.clan_relations
            .can_attack_inside(attacker_clan, defender_clan)
    }

    fn can_attack_outside_clan_area(&self, attacker_clan: u16, defender_clan: u16) -> bool {
        self.clan_relations
            .can_attack_outside(attacker_clan, defender_clan)
    }
}

#[cfg(test)]
mod look_tests {
    use super::*;

    #[test]
    fn format_look_note_line_matches_c_list_punishment_shape() {
        let note = PunishmentNote {
            level: 3,
            exp: 400,
            karma: 4,
            reason: "being mean".to_string(),
        };
        // 1_000_000_000 unix seconds = 2001-09-09 01:46:40 UTC.
        let line = format_look_note_line(7, &note, "Godmode", 1_000_000_000);
        assert_eq!(
            line,
            "P7: Level: 3, Exp: 400, Karma: 4, Creator: Godmode, Date: 09/09/2001 01:46:40, Reason: being mean"
        );
    }

    #[tokio::test]
    async fn no_look_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_look_events(&mut world, &None, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_look_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_look_command(CharacterId(1), "Baddie");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_look_events(&mut world, &None, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_look_requests().is_empty());
    }
}

#[cfg(test)]
mod values_tests {
    use super::*;

    #[tokio::test]
    async fn no_values_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        let applied = apply_values_events(&mut world, &mut runtime, &None, 1, 1, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_values_queue_without_a_reply() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        world.queue_values_command(CharacterId(1), "Someone");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_values_events(&mut world, &mut runtime, &None, 1, 1, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_values_requests().is_empty());
    }
}

#[cfg(test)]
mod allow_tests {
    use super::*;

    #[tokio::test]
    async fn no_allow_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_allow_events(&mut world, &None, 1).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_allow_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_allow_command(CharacterId(1), "Someone");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_allow_events(&mut world, &None, 1).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_allow_requests().is_empty());
    }
}
