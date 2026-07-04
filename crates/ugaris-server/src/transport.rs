use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TransportDestination {
    name: &'static str,
    x: u16,
    y: u16,
    area: u16,
}

pub(crate) const LEGACY_TRANSPORT_DESTINATIONS: [TransportDestination; 26] = [
    TransportDestination {
        name: "Cameron",
        x: 139,
        y: 75,
        area: 1,
    },
    TransportDestination {
        name: "Chapel",
        x: 139,
        y: 75,
        area: 1,
    },
    TransportDestination {
        name: "Aston",
        x: 129,
        y: 201,
        area: 3,
    },
    TransportDestination {
        name: "Tribe of the Isara",
        x: 239,
        y: 249,
        area: 6,
    },
    TransportDestination {
        name: "Tribe of the Cerasa",
        x: 92,
        y: 164,
        area: 6,
    },
    TransportDestination {
        name: "Maze of the Cerasa",
        x: 49,
        y: 135,
        area: 6,
    },
    TransportDestination {
        name: "Defense Tunnels of the Cerasa",
        x: 14,
        y: 114,
        area: 6,
    },
    TransportDestination {
        name: "Zalina Entrance",
        x: 5,
        y: 4,
        area: 6,
    },
    TransportDestination {
        name: "Tribe of the Zalina",
        x: 172,
        y: 36,
        area: 6,
    },
    TransportDestination {
        name: "Teufelheim",
        x: 225,
        y: 249,
        area: 34,
    },
    TransportDestination {
        name: "Aston Mines",
        x: 57,
        y: 124,
        area: 3,
    },
    TransportDestination {
        name: "*empty*",
        x: 0,
        y: 0,
        area: 0,
    },
    TransportDestination {
        name: "Ice 1",
        x: 93,
        y: 102,
        area: 10,
    },
    TransportDestination {
        name: "Ice 2",
        x: 11,
        y: 113,
        area: 10,
    },
    TransportDestination {
        name: "Ice 3",
        x: 241,
        y: 87,
        area: 10,
    },
    TransportDestination {
        name: "Ice 4",
        x: 213,
        y: 156,
        area: 11,
    },
    TransportDestination {
        name: "Ice 5",
        x: 189,
        y: 80,
        area: 11,
    },
    TransportDestination {
        name: "Nomad Plains",
        x: 16,
        y: 124,
        area: 19,
    },
    TransportDestination {
        name: "*empty*",
        x: 0,
        y: 0,
        area: 0,
    },
    TransportDestination {
        name: "*empty*",
        x: 0,
        y: 0,
        area: 0,
    },
    TransportDestination {
        name: "Forest",
        x: 181,
        y: 117,
        area: 16,
    },
    TransportDestination {
        name: "Exkordon",
        x: 65,
        y: 106,
        area: 17,
    },
    TransportDestination {
        name: "Brannington",
        x: 202,
        y: 226,
        area: 29,
    },
    TransportDestination {
        name: "Grimroot",
        x: 210,
        y: 246,
        area: 31,
    },
    TransportDestination {
        name: "Caligar",
        x: 230,
        y: 62,
        area: 36,
    },
    TransportDestination {
        name: "Arkhata",
        x: 28,
        y: 20,
        area: 37,
    },
];

pub(crate) const LEGACY_TRANSPORT_CLAN_DESTINATIONS: [TransportDestination; 32] = [
    TransportDestination {
        name: "Clan1",
        x: 28,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan2",
        x: 59,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan3",
        x: 90,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan4",
        x: 121,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan5",
        x: 152,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan6",
        x: 183,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan7",
        x: 214,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan8",
        x: 245,
        y: 18,
        area: 3,
    },
    TransportDestination {
        name: "Clan9",
        x: 28,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan10",
        x: 59,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan11",
        x: 90,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan12",
        x: 121,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan13",
        x: 152,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan14",
        x: 183,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan15",
        x: 214,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan16",
        x: 245,
        y: 38,
        area: 3,
    },
    TransportDestination {
        name: "Clan17",
        x: 28,
        y: 58,
        area: 3,
    },
    TransportDestination {
        name: "Clan18",
        x: 59,
        y: 58,
        area: 3,
    },
    TransportDestination {
        name: "Clan19",
        x: 90,
        y: 58,
        area: 3,
    },
    TransportDestination {
        name: "Clan20",
        x: 121,
        y: 58,
        area: 3,
    },
    TransportDestination {
        name: "Clan21",
        x: 152,
        y: 58,
        area: 3,
    },
    TransportDestination {
        name: "Clan22",
        x: 183,
        y: 58,
        area: 3,
    },
    TransportDestination {
        name: "Clan23",
        x: 28,
        y: 78,
        area: 3,
    },
    TransportDestination {
        name: "Clan24",
        x: 59,
        y: 78,
        area: 3,
    },
    TransportDestination {
        name: "Clan25",
        x: 90,
        y: 78,
        area: 3,
    },
    TransportDestination {
        name: "Clan26",
        x: 121,
        y: 78,
        area: 3,
    },
    TransportDestination {
        name: "Clan27",
        x: 152,
        y: 78,
        area: 3,
    },
    TransportDestination {
        name: "Clan28",
        x: 183,
        y: 78,
        area: 3,
    },
    TransportDestination {
        name: "Clan29",
        x: 28,
        y: 251,
        area: 3,
    },
    TransportDestination {
        name: "Clan30",
        x: 59,
        y: 251,
        area: 3,
    },
    TransportDestination {
        name: "Clan31",
        x: 90,
        y: 251,
        area: 3,
    },
    TransportDestination {
        name: "Clan32",
        x: 28,
        y: 231,
        area: 3,
    },
];

/// C `may_enter_clan` (`clan.c:881-905`), called from `transport.c:185-223`.
/// Delegates to `ClanRelations::may_enter`: own-clan entry is always
/// allowed, non-members are always rejected, a never-founded/deleted clan
/// hall admits nobody, and otherwise only an `Alliance` relation (from the
/// target clan's perspective, matching C's
/// `clan[nr].status.current_relation[cnr]`) grants access.
pub(crate) fn may_enter_clan(world: &World, character: &Character, clan: u16) -> bool {
    (1..=32).contains(&clan)
        && world
            .clan_registry
            .relations()
            .may_enter(character.clan, clan)
}

pub(crate) fn transport_clan_access(world: &World, character_id: CharacterId) -> [u8; 4] {
    let Some(character) = world.characters.get(&character_id) else {
        return [0; 4];
    };
    let mut access = [0_u8; 4];
    for clan in 1..=32_u16 {
        if may_enter_clan(world, character, clan) {
            let index = (clan - 1) as usize;
            access[index / 8] |= 1_u8 << (index % 8);
        }
    }
    access
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TransportTravelResult {
    SameArea {
        x: u16,
        y: u16,
        mirror: u32,
    },
    CrossArea {
        area: u16,
        x: u16,
        y: u16,
        mirror: u32,
    },
    Busy,
    Blocked(String),
    Bug(String),
}

pub(crate) fn resolve_transport_travel(
    world: &World,
    player: &PlayerRuntime,
    character_id: CharacterId,
    current_area: u16,
    spec: i32,
) -> TransportTravelResult {
    resolve_transport_travel_with_random(
        world,
        player,
        character_id,
        current_area,
        spec,
        runtime_random_below,
    )
}

pub(crate) fn resolve_transport_travel_with_random(
    world: &World,
    player: &PlayerRuntime,
    character_id: CharacterId,
    current_area: u16,
    spec: i32,
    mut random_below: impl FnMut(i32) -> i32,
) -> TransportTravelResult {
    let nr = (spec & 255) - 1;
    let mirror = match spec / 256 {
        1..=26 => (spec / 256) as u32,
        _ => (random_below(26).clamp(0, 25) + 1) as u32,
    };

    if (64..96).contains(&nr) {
        let clan = (nr - 63) as u16;
        if !world
            .characters
            .get(&character_id)
            .is_some_and(|character| may_enter_clan(world, character, clan))
        {
            return TransportTravelResult::Blocked(format!("You may not enter ({}).", clan));
        }
        let destination = LEGACY_TRANSPORT_CLAN_DESTINATIONS[(clan - 1) as usize];
        if destination.area != current_area {
            return TransportTravelResult::CrossArea {
                area: destination.area,
                x: destination.x,
                y: destination.y,
                mirror,
            };
        }
        return TransportTravelResult::SameArea {
            x: destination.x,
            y: destination.y,
            mirror,
        };
    }

    if !(0..64).contains(&nr) {
        return TransportTravelResult::Bug("You've confused me. (BUG #1123)".to_string());
    }

    let point = nr as usize;
    let bit = 1_u64 << point;
    let Some(destination) = LEGACY_TRANSPORT_DESTINATIONS.get(point).copied() else {
        return TransportTravelResult::Bug(format!("Nothing happens - BUG ({nr},#2)."));
    };
    if player.transport_seen & bit == 0 {
        return TransportTravelResult::Blocked(format!(
            "You've never been to {} before. You cannot go there.",
            destination.name
        ));
    }
    if point == 22
        && !world
            .characters
            .get(&character_id)
            .is_some_and(|character| character.flags.contains(CharacterFlags::ARCH))
    {
        return TransportTravelResult::Blocked("Sorry, Arches only!".to_string());
    }
    if destination.x < 1 || destination.x > 254 || destination.y < 1 || destination.y > 254 {
        return TransportTravelResult::Bug(format!(
            "Nothing happens - BUG ({},{},{}).",
            destination.x, destination.y, destination.area
        ));
    }
    if destination.area != current_area {
        return TransportTravelResult::CrossArea {
            area: destination.area,
            x: destination.x,
            y: destination.y,
            mirror,
        };
    }
    TransportTravelResult::SameArea {
        x: destination.x,
        y: destination.y,
        mirror,
    }
}

pub(crate) fn apply_transport_travel(
    world: &mut World,
    player: &PlayerRuntime,
    character_id: CharacterId,
    current_area: u16,
    spec: i32,
) -> TransportTravelResult {
    let resolved = resolve_transport_travel(world, player, character_id, current_area, spec);
    if let TransportTravelResult::SameArea { x, y, mirror } = resolved {
        if world.teleport_character_same_area(character_id, x, y, false) {
            TransportTravelResult::SameArea { x, y, mirror }
        } else {
            TransportTravelResult::Busy
        }
    } else {
        resolved
    }
}
