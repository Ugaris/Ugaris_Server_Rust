use super::*;

#[test]
fn toplist_driver_dispatches_for_players_only() {
    let mut character = character(1);
    let mut toplist = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TOPLIST);
    let request = ItemDriverRequest::Driver {
        driver: IDR_TOPLIST,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut toplist, request, 1, false),
        ItemDriverOutcome::ArenaToplist {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    character.id = CharacterId(0);
    assert_eq!(
        execute_item_driver(&mut character, &mut toplist, request, 1, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn arena_toplist_lines_match_legacy_rank_window() {
    let entries: Vec<ArenaToplistEntry> = (0..20)
        .map(|index| ArenaToplistEntry {
            name: format!("Fighter{index}"),
            score: 2000 - index * 100,
        })
        .collect();

    let lines = arena_toplist_lines(&entries, 650, 3, 2, 5);

    assert_eq!(lines[0], "1: Fighter0 2000");
    assert_eq!(lines[9], "10: Fighter9 1100");
    assert_eq!(lines[10], "11: Fighter10 1000");
    assert_eq!(lines[14], "15: Fighter14 600");
    assert_eq!(
        lines.last().unwrap(),
        "Your score is 650, you have won 3 fights and lost 2 fights."
    );
}

#[test]
fn arena_toplist_lines_use_legacy_newcomer_score() {
    let entries = vec![ArenaToplistEntry {
        name: "Champion".to_string(),
        score: 42,
    }];

    let lines = arena_toplist_lines(&entries, 500, 0, 0, 0);

    assert_eq!(lines[0], "1: Champion 42");
    assert_eq!(
        lines[1],
        "Your score is -2000, you have won 0 fights and lost 0 fights."
    );
}
