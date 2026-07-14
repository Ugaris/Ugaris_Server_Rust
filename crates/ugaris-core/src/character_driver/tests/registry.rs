use super::*;

#[test]
fn legacy_dispatch_type_constants_match_c_libload() {
    assert_eq!(CDT_DRIVER, 0);
    assert_eq!(CDT_ITEM, 1);
    assert_eq!(CDT_DEAD, 2);
    assert_eq!(CDT_RESPAWN, 3);
    assert_eq!(CDT_SPECIAL, 4);
}

#[test]
fn parse_clanclerk_driver_args_reads_bare_clan_number() {
    assert_eq!(parse_clanclerk_driver_args("5").clan, 5);
    assert_eq!(parse_clanclerk_driver_args(" 12 ").clan, 12);
    assert_eq!(parse_clanclerk_driver_args("").clan, 0);
    assert_eq!(parse_clanclerk_driver_args("not-a-number").clan, 0);
}

#[test]
fn cdr_clanclerk_matches_c_drvlib() {
    assert_eq!(CDR_CLANMASTER, 27);
    assert_eq!(CDR_CLANCLERK, 28);
}

#[test]
fn cdr_clubmaster_matches_c_drvlib() {
    assert_eq!(CDR_CLUBMASTER, 113);
}

#[test]
fn parse_clubmaster_driver_args_reads_dir() {
    assert_eq!(parse_clubmaster_driver_args("dir=3;").dir, 3);
    assert_eq!(parse_clubmaster_driver_args("").dir, 0);
}

#[test]
fn cdr_arena_constants_match_c_drvlib() {
    assert_eq!(CDR_ARENAMASTER, 48);
    assert_eq!(CDR_ARENAFIGHTER, 49);
    assert_eq!(CDR_ARENAMANAGER, 50);
}

#[test]
fn parse_arena_manager_driver_args_reads_real_zone_file_arg() {
    // Verbatim `arg=` from `ugaris_data/zones/3/above3_generic.chr`.
    let data = parse_arena_manager_driver_args(
        "arenax=233;arenay=122;arenafx=230;arenafy=119;arenatx=242;arenaty=125;",
    );
    assert_eq!(data.arena_x, 233);
    assert_eq!(data.arena_y, 122);
    assert_eq!(data.arena_fx, 230);
    assert_eq!(data.arena_fy, 119);
    assert_eq!(data.arena_tx, 242);
    assert_eq!(data.arena_ty, 125);
    assert_eq!(data.renter, None);
    assert!(data.invite.is_empty());
}

#[test]
fn parse_arena_manager_driver_args_ignores_unknown_names() {
    let data = parse_arena_manager_driver_args("arenax=5;bogus=9;arenay=6;");
    assert_eq!(data.arena_x, 5);
    assert_eq!(data.arena_y, 6);
}

#[test]
fn notify_constants_match_c_notify_header() {
    assert_eq!(NT_CHAR, 1);
    assert_eq!(NT_ITEM, 2);
    assert_eq!(NT_GOTHIT, 3);
    assert_eq!(NT_DIDHIT, 4);
    assert_eq!(NT_SEEHIT, 5);
    assert_eq!(NT_DEAD, 6);
    assert_eq!(NT_SPELL, 7);
    assert_eq!(NT_GIVE, 8);
    assert_eq!(NT_CREATE, 9);
    assert_eq!(NT_TEXT, 200);
    assert_eq!(NT_NPC, 300);
    assert_eq!(NTID_MERCHANT, 1);
    assert_eq!(NTID_GLADIATOR, 16);
}

#[test]
fn base_character_driver_ids_match_c_drvlib() {
    assert_eq!(CDR_LOSTCON, 5);
    assert_eq!(CDR_SIMPLEBADDY, 7);
    assert_eq!(CDR_MACRO, 37);
    assert_eq!(CDR_SWAMPCLARA, 54);
    assert_eq!(CDR_SWAMPMONSTER, 56);
    assert_eq!(CDR_PALACEISLENA, 57);
    assert_eq!(CDR_TWOBARKEEPER, 63);
    assert_eq!(CDR_TWOSERVANT, 65);
    assert_eq!(CDR_TWOTHIEFGUARD, 66);
    assert_eq!(CDR_TWOTHIEFMASTER, 67);
    assert_eq!(CDR_TWOROBBER, 68);
    assert_eq!(CDR_TWOSANWYN, 69);
    assert_eq!(CDR_TWOSKELLY, 70);
    assert_eq!(CDR_TWOALCHEMIST, 71);
    assert_eq!(CDR_TRADER, 72);
    assert_eq!(CDR_LQNPC, 74);
    assert_eq!(CDR_JANITOR, 85);
    assert_eq!(CDR_TEUFELDEMON, 114);
    assert_eq!(CDR_TEUFELGAMBLER, 115);
    assert_eq!(CDR_TEUFELQUEST, 116);
    assert_eq!(CDR_TEUFELRAT, 117);
    assert_eq!(CDR_CALIGARGUARD, 118);
    assert_eq!(CDR_CALIGARGLORI, 119);
    assert_eq!(CDR_CALIGARARQUIN, 120);
    assert_eq!(CDR_CALIGARSMITH, 121);
    assert_eq!(CDR_CALIGARHOMDEN, 122);
    assert_eq!(CDR_CALIGARGUARD2, 123);
    assert_eq!(CDR_CALIGARSKELLY, 124);
    assert_eq!(CDR_LAB2UNDEAD, 198);
    assert_eq!(DRD_SIMPLEBADDYDRIVER, 0x0100_0013);
    assert_eq!(
        CharacterDriverKind::SimpleBaddy.legacy_id(),
        CDR_SIMPLEBADDY
    );
    assert_eq!(CharacterDriverKind::Macro.legacy_id(), CDR_MACRO);
    assert_eq!(CharacterDriverKind::SwampClara.legacy_id(), CDR_SWAMPCLARA);
    assert_eq!(
        CharacterDriverKind::SwampMonster.legacy_id(),
        CDR_SWAMPMONSTER
    );
    assert_eq!(
        CharacterDriverKind::PalaceIslena.legacy_id(),
        CDR_PALACEISLENA
    );
    assert_eq!(CharacterDriverKind::Trader.legacy_id(), CDR_TRADER);
    assert_eq!(CharacterDriverKind::LqNpc.legacy_id(), CDR_LQNPC);
    assert_eq!(CharacterDriverKind::Janitor.legacy_id(), CDR_JANITOR);
    assert_eq!(
        CharacterDriverKind::TeufelDemon.legacy_id(),
        CDR_TEUFELDEMON
    );
    assert_eq!(
        CharacterDriverKind::TeufelGambler.legacy_id(),
        CDR_TEUFELGAMBLER
    );
    assert_eq!(
        CharacterDriverKind::TeufelQuest.legacy_id(),
        CDR_TEUFELQUEST
    );
    assert_eq!(CharacterDriverKind::TeufelRat.legacy_id(), CDR_TEUFELRAT);
    assert_eq!(
        CharacterDriverKind::CaligarSkelly.legacy_id(),
        CDR_CALIGARSKELLY
    );
    assert_eq!(CharacterDriverKind::Lab2Undead.legacy_id(), CDR_LAB2UNDEAD);
    assert_eq!(DRD_CLARADRIVER, 0x0100_0059);
    assert_eq!(DRD_SKELLYDRIVER, 0x0100_006a);
    assert_eq!(DRD_LAB2_UNDEAD, 0x0200_0001);
}

#[test]
fn two_sanwyn_driver_state_matches_legacy_runtime_data_shape() {
    let mut data = TwoSanwynDriverData::default();
    assert_eq!(data.last_talk_tick, 0);
    assert_eq!(data.current_victim, None);

    data.last_talk_tick = 111;
    data.current_victim = Some(CharacterId(12));
    assert_eq!(
        CharacterDriverState::TwoSanwyn(data),
        CharacterDriverState::TwoSanwyn(TwoSanwynDriverData {
            last_talk_tick: 111,
            current_victim: Some(CharacterId(12)),
        })
    );
}

#[test]
fn two_thiefguard_driver_state_matches_legacy_runtime_data_shape() {
    let mut data = TwoThiefGuardDriverData::default();
    assert_eq!(data.last_talk_tick, 0);
    assert_eq!(data.current_victim, None);

    data.last_talk_tick = 111;
    data.current_victim = Some(CharacterId(12));
    assert_eq!(
        CharacterDriverState::TwoThiefGuard(data),
        CharacterDriverState::TwoThiefGuard(TwoThiefGuardDriverData {
            last_talk_tick: 111,
            current_victim: Some(CharacterId(12)),
        })
    );
}

#[test]
fn two_thiefmaster_driver_state_matches_legacy_runtime_data_shape() {
    let mut data = TwoThiefMasterDriverData::default();
    assert_eq!(data.last_talk_tick, 0);
    assert_eq!(data.current_victim, None);

    data.last_talk_tick = 111;
    data.current_victim = Some(CharacterId(12));
    assert_eq!(
        CharacterDriverState::TwoThiefMaster(data),
        CharacterDriverState::TwoThiefMaster(TwoThiefMasterDriverData {
            last_talk_tick: 111,
            current_victim: Some(CharacterId(12)),
        })
    );
}

#[test]
fn two_skelly_driver_state_matches_legacy_runtime_data_shape() {
    let mut data = TwoSkellyDriverData::default();
    assert_eq!(data.last_talk_tick, 0);
    assert_eq!(data.current_victim, None);
    assert_eq!(data.alive_tick, 0);

    data.last_talk_tick = 111;
    data.current_victim = Some(CharacterId(12));
    data.alive_tick = 222;
    assert_eq!(
        CharacterDriverState::TwoSkelly(data),
        CharacterDriverState::TwoSkelly(TwoSkellyDriverData {
            last_talk_tick: 111,
            current_victim: Some(CharacterId(12)),
            alive_tick: 222,
        })
    );
}

#[test]
fn two_alchemist_driver_state_matches_legacy_runtime_data_shape() {
    let mut data = TwoAlchemistDriverData::default();
    assert_eq!(data.last_talk_tick, 0);
    assert_eq!(data.current_victim, None);

    data.last_talk_tick = 111;
    data.current_victim = Some(CharacterId(12));
    assert_eq!(
        CharacterDriverState::TwoAlchemist(data),
        CharacterDriverState::TwoAlchemist(TwoAlchemistDriverData {
            last_talk_tick: 111,
            current_victim: Some(CharacterId(12)),
        })
    );
}

#[test]
fn two_barkeeper_driver_state_matches_legacy_runtime_data_shape() {
    let mut data = TwoBarkeeperDriverData::default();
    assert_eq!(data.last_talk_tick, 0);
    assert_eq!(data.current_victim, None);

    data.last_talk_tick = 111;
    data.current_victim = Some(CharacterId(12));
    assert_eq!(
        CharacterDriverState::TwoBarkeeper(data),
        CharacterDriverState::TwoBarkeeper(TwoBarkeeperDriverData {
            last_talk_tick: 111,
            current_victim: Some(CharacterId(12)),
        })
    );
}

#[test]
fn two_servant_driver_state_matches_legacy_runtime_data_shape() {
    let mut data = TwoServantDriverData::default();
    assert_eq!(data.last_talk_tick, 0);
    assert_eq!(data.current_victim, None);
    assert_eq!(data.current_state, 0);
    assert_eq!(data.nr, 0);
    assert_eq!(data.lastalert, 0);

    data.last_talk_tick = 111;
    data.current_victim = Some(CharacterId(12));
    data.current_state = 1;
    data.nr = 4;
    data.lastalert = 222;
    assert_eq!(
        CharacterDriverState::TwoServant(data),
        CharacterDriverState::TwoServant(TwoServantDriverData {
            last_talk_tick: 111,
            current_victim: Some(CharacterId(12)),
            current_state: 1,
            nr: 4,
            lastalert: 222,
        })
    );
}

#[test]
fn parse_two_servant_driver_args_parses_nr() {
    let data = crate::world::npc::area17::servant::parse_two_servant_driver_args("nr=4;");
    assert_eq!(data.nr, 4);
    assert_eq!(data.current_state, 0);
    assert_eq!(data.last_talk_tick, 0);
}

#[test]
fn clara_driver_state_matches_legacy_runtime_data_shape() {
    let mut data = ClaraDriverData::default();
    assert_eq!(data.last_talk, 0);
    assert_eq!(data.current_victim, None);

    data.last_talk = 1234;
    data.current_victim = Some(CharacterId(77));
    assert_eq!(
        CharacterDriverState::Clara(data),
        CharacterDriverState::Clara(ClaraDriverData {
            last_talk: 1234,
            current_victim: Some(CharacterId(77)),
        })
    );
}

#[test]
fn known_base_tick_drivers_are_handled_like_c_ch_driver() {
    for (driver, kind) in [
        (CDR_SIMPLEBADDY, CharacterDriverKind::SimpleBaddy),
        (CDR_MACRO, CharacterDriverKind::Macro),
        (CDR_SWAMPCLARA, CharacterDriverKind::SwampClara),
        (CDR_SWAMPMONSTER, CharacterDriverKind::SwampMonster),
        (CDR_PALACEISLENA, CharacterDriverKind::PalaceIslena),
        (CDR_TRADER, CharacterDriverKind::Trader),
        (CDR_LQNPC, CharacterDriverKind::LqNpc),
        (CDR_JANITOR, CharacterDriverKind::Janitor),
        (CDR_TEUFELDEMON, CharacterDriverKind::TeufelDemon),
        (CDR_TEUFELGAMBLER, CharacterDriverKind::TeufelGambler),
        (CDR_TEUFELQUEST, CharacterDriverKind::TeufelQuest),
        (CDR_TEUFELRAT, CharacterDriverKind::TeufelRat),
        (CDR_LAB2UNDEAD, CharacterDriverKind::Lab2Undead),
    ] {
        let outcome = execute_character_driver(driver, 7, 11);
        assert_eq!(
            outcome,
            CharacterDriverOutcome::HandledStub {
                kind,
                call: CharacterDriverCall::Tick {
                    ret: 7,
                    last_action: 11,
                },
            }
        );
        assert_eq!(outcome.legacy_return_code(), 1);
    }
}

#[test]
fn known_base_death_and_respawn_drivers_are_handled_like_c() {
    let simple_died = execute_character_died_driver(CDR_SIMPLEBADDY, 123);
    assert_eq!(
        simple_died,
        CharacterDriverOutcome::SimpleBaddyDeath {
            killer_character_id: 123,
        }
    );
    assert_eq!(simple_died.legacy_return_code(), 1);

    let died = execute_character_died_driver(CDR_JANITOR, 123);
    assert_eq!(
        died,
        CharacterDriverOutcome::HandledStub {
            kind: CharacterDriverKind::Janitor,
            call: CharacterDriverCall::Died {
                killer_character_id: 123,
            },
        }
    );
    assert_eq!(died.legacy_return_code(), 1);

    let islena_died = execute_character_died_driver(CDR_PALACEISLENA, 123);
    assert_eq!(
        islena_died,
        CharacterDriverOutcome::HandledStub {
            kind: CharacterDriverKind::PalaceIslena,
            call: CharacterDriverCall::Died {
                killer_character_id: 123,
            },
        }
    );
    assert_eq!(islena_died.legacy_return_code(), 1);

    let clara_died = execute_character_died_driver(CDR_SWAMPCLARA, 123);
    assert_eq!(
        clara_died,
        CharacterDriverOutcome::HandledStub {
            kind: CharacterDriverKind::SwampClara,
            call: CharacterDriverCall::Died {
                killer_character_id: 123,
            },
        }
    );
    assert_eq!(clara_died.legacy_return_code(), 1);

    let swamp_monster_died = execute_character_died_driver(CDR_SWAMPMONSTER, 123);
    assert_eq!(
        swamp_monster_died,
        CharacterDriverOutcome::HandledStub {
            kind: CharacterDriverKind::SwampMonster,
            call: CharacterDriverCall::Died {
                killer_character_id: 123,
            },
        }
    );
    assert_eq!(swamp_monster_died.legacy_return_code(), 1);

    let simple_respawn = execute_character_respawn_driver(CDR_SIMPLEBADDY);
    assert_eq!(
        simple_respawn,
        CharacterDriverOutcome::HandledStub {
            kind: CharacterDriverKind::SimpleBaddy,
            call: CharacterDriverCall::Respawn,
        }
    );
    assert_eq!(simple_respawn.legacy_return_code(), 1);

    let respawn = execute_character_respawn_driver(CDR_TRADER);
    assert_eq!(
        respawn,
        CharacterDriverOutcome::HandledStub {
            kind: CharacterDriverKind::Trader,
            call: CharacterDriverCall::Respawn,
        }
    );
    assert_eq!(respawn.legacy_return_code(), 1);

    let islena_respawn = execute_character_respawn_driver(CDR_PALACEISLENA);
    assert_eq!(
        islena_respawn,
        CharacterDriverOutcome::HandledStub {
            kind: CharacterDriverKind::PalaceIslena,
            call: CharacterDriverCall::Respawn,
        }
    );
    assert_eq!(islena_respawn.legacy_return_code(), 1);

    let clara_respawn = execute_character_respawn_driver(CDR_SWAMPCLARA);
    assert_eq!(
        clara_respawn,
        CharacterDriverOutcome::HandledStub {
            kind: CharacterDriverKind::SwampClara,
            call: CharacterDriverCall::Respawn,
        }
    );
    assert_eq!(clara_respawn.legacy_return_code(), 1);

    let swamp_monster_respawn = execute_character_respawn_driver(CDR_SWAMPMONSTER);
    assert_eq!(
        swamp_monster_respawn,
        CharacterDriverOutcome::HandledStub {
            kind: CharacterDriverKind::SwampMonster,
            call: CharacterDriverCall::Respawn,
        }
    );
    assert_eq!(swamp_monster_respawn.legacy_return_code(), 1);

    let lab2_undead_died = execute_character_died_driver(CDR_LAB2UNDEAD, 123);
    assert_eq!(
        lab2_undead_died,
        CharacterDriverOutcome::HandledStub {
            kind: CharacterDriverKind::Lab2Undead,
            call: CharacterDriverCall::Died {
                killer_character_id: 123,
            },
        }
    );
    assert_eq!(lab2_undead_died.legacy_return_code(), 1);

    let lab2_undead_respawn = execute_character_respawn_driver(CDR_LAB2UNDEAD);
    assert_eq!(
        lab2_undead_respawn,
        CharacterDriverOutcome::HandledStub {
            kind: CharacterDriverKind::Lab2Undead,
            call: CharacterDriverCall::Respawn,
        }
    );
    assert_eq!(lab2_undead_respawn.legacy_return_code(), 1);
}

#[test]
fn unknown_character_driver_returns_legacy_zero() {
    let outcome = execute_character_driver(999, 0, 0);
    assert_eq!(
        outcome,
        CharacterDriverOutcome::Unsupported {
            driver: 999,
            call: CharacterDriverCall::Tick {
                ret: 0,
                last_action: 0,
            },
        }
    );
    assert_eq!(outcome.legacy_return_code(), 0);
}
