//! The generic `CDR_SIMPLEBADDY` fight driver: driver data, legacy arg
//! parsing, message processing and enemy bookkeeping.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SimpleBaddyDriverData {
    pub startdist: i32,
    pub chardist: i32,
    pub stopdist: i32,
    #[serde(default)]
    pub home_x: u16,
    #[serde(default)]
    pub home_y: u16,
    pub aggressive: i32,
    pub helper: i32,
    pub scavenger: i32,
    pub dir: i32,
    pub dayx: i32,
    pub dayy: i32,
    pub daydir: i32,
    pub nightx: i32,
    pub nighty: i32,
    pub nightdir: i32,
    pub teleport: i32,
    pub helpid: i32,
    pub creation_time: i32,
    pub notsecure: i32,
    pub mindist: i32,
    pub lastfight: i32,
    #[serde(default)]
    pub last_hit: i32,
    #[serde(default)]
    pub pending_bless_friend: Option<CharacterId>,
    pub poison_power: i32,
    pub poison_chance: i32,
    pub poison_type: i32,
    pub drinkspecial: i32,
    pub drink_inventory_potions: i32,
    #[serde(default)]
    pub enemies: Vec<SimpleBaddyEnemy>,
    /// C `CDR_FDEMON_DEMON`'s own separate `struct fdemon_data.gohome`
    /// slot (`fdemon.c:2670-2673`, `DRD_FDEMONDATA`) - a sticky "walk back
    /// toward `tmpx`/`tmpy` (this port's `Character::rest_x`/`rest_y`)"
    /// flag set once the demon strays too far from home and cleared once
    /// it's back within range. Bolted onto this reused struct rather than
    /// getting its own `CharacterDriverState` variant, same precedent as
    /// `CDR_PENTER`/`CDR_DUNGEONFIGHTER` reusing `SimpleBaddy` wholesale
    /// (see `CDR_FDEMON_DEMON`'s own doc comment); C's `dat->dir` field is
    /// reused directly via this struct's own pre-existing `dir` field for
    /// the same reason. Only ever read/written by
    /// `world::npc::area8::fdemon_demon`.
    #[serde(default)]
    pub fdemon_gohome: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SimpleBaddyEnemy {
    pub target_id: CharacterId,
    pub priority: i32,
    pub last_seen_tick: i32,
    #[serde(default)]
    pub visible: bool,
    #[serde(default)]
    pub last_x: u16,
    #[serde(default)]
    pub last_y: u16,
}
/// C `struct fight_driver_data` (`src/common/fight.h:27-37`), stored via
/// `set_data(cn, DRD_FIGHTDRIVER, ...)` - a slot independent of whichever
/// `driver`/`driver_state` a character currently has (C's `set_data` lets
/// one character hold named data blobs for several drivers/subsystems at
/// once; the `simple_baddy` driver's own `startdist`/`chardist`/`stopdist`/
/// `lastfight` fields on [`SimpleBaddyDriverData`] are a *different*,
/// simple_baddy-owned copy only used to seed this one once at creation via
/// `fight_driver_set_dist`, `simple_baddy.c:189` - see
/// `apply_simple_baddy_create_message`). Lives on the dedicated
/// [`crate::entity::Character::fight_driver`] field, mirroring the
/// existing `Character::dungeonfighter` precedent, so any character
/// (SimpleBaddy NPC, lostcon corpse, or a normal playing character with a
/// `no*`/`auto*` toggle set) can drive `fight_driver_attack_enemy`'s
/// enemy-tracking without needing a `SimpleBaddyDriverData` of its own.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct FightDriverData {
    /// C `struct person enemy[10]`.
    #[serde(default)]
    pub enemies: Vec<SimpleBaddyEnemy>,
    /// C `start_dist`: distance from home at which to start attacking.
    #[serde(default)]
    pub start_dist: i32,
    /// C `stop_dist`: distance from home at which to stop attacking.
    #[serde(default)]
    pub stop_dist: i32,
    /// C `char_dist`: distance from the character we start attacking.
    #[serde(default)]
    pub char_dist: i32,
    /// C `home_x`/`home_y`: position `start_dist`/`stop_dist` are measured
    /// from; falls back to the respawn point (then current position) when
    /// zero, exactly like `fight_driver_dist_from_home`.
    #[serde(default)]
    pub home_x: u16,
    #[serde(default)]
    pub home_y: u16,
    /// C `lasthit`: tick of the last `fight_driver_note_hit` call, read by
    /// `fight_driver_regen_value`'s post-hit regen-suppression window.
    #[serde(default)]
    pub last_hit: i32,
}
impl Default for SimpleBaddyDriverData {
    fn default() -> Self {
        Self {
            startdist: 20,
            chardist: 0,
            stopdist: 40,
            home_x: 0,
            home_y: 0,
            aggressive: 0,
            helper: 0,
            scavenger: 0,
            dir: 3,
            dayx: 0,
            dayy: 0,
            daydir: 0,
            nightx: 0,
            nighty: 0,
            nightdir: 0,
            teleport: 0,
            helpid: 0,
            creation_time: 0,
            notsecure: 0,
            mindist: 0,
            lastfight: 0,
            last_hit: 0,
            pending_bless_friend: None,
            poison_power: 0,
            poison_chance: 0,
            poison_type: 0,
            drinkspecial: 0,
            drink_inventory_potions: 0,
            enemies: Vec::new(),
            fdemon_gohome: false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownSimpleBaddyArgument {
    pub name: String,
    pub value: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleBaddyParseResult {
    pub data: SimpleBaddyDriverData,
    pub unknown: Vec<UnknownSimpleBaddyArgument>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleBaddyMessageOutcome {
    UseInventoryPotion {
        item_id: ItemId,
        reason: PotionUseReason,
    },
    BlessFriend {
        target_id: crate::ids::CharacterId,
    },
    PoisonHit {
        target_id: CharacterId,
        power: u16,
        poison_type: u16,
        chance: i32,
    },
    AddEnemy {
        caller_id: CharacterId,
        target_id: CharacterId,
    },
    RemoveEnemy {
        target_id: CharacterId,
    },
    StandardAggro {
        target_id: CharacterId,
        priority: i32,
        require_visible: bool,
        hurtme: bool,
    },
    StandardSeenHit {
        attacker_id: CharacterId,
        victim_id: CharacterId,
    },
    TextNotification {
        speaker_id: CharacterId,
        text_token: i32,
        text: Option<String>,
    },
    NoteHit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PotionUseReason {
    LowHp,
    LowMana,
}
pub fn parse_simple_baddy_driver_args(args: &str) -> SimpleBaddyParseResult {
    let mut data = SimpleBaddyDriverData::default();
    let mut unknown = Vec::new();
    let mut rest = args;

    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        let parsed = value.parse::<i32>().unwrap_or(0);
        match name {
            "aggressive" => data.aggressive = parsed,
            "scavenger" => data.scavenger = parsed,
            "helper" => data.helper = parsed,
            "startdist" => data.startdist = parsed,
            "chardist" => data.chardist = parsed,
            "stopdist" => data.stopdist = parsed,
            "dir" => data.dir = parsed,
            "dayx" => data.dayx = parsed,
            "dayy" => data.dayy = parsed,
            "daydir" => data.daydir = parsed,
            "nightx" => data.nightx = parsed,
            "nighty" => data.nighty = parsed,
            "nightdir" => data.nightdir = parsed,
            "teleport" => data.teleport = parsed,
            "helpid" => data.helpid = parsed,
            "notsecure" => data.notsecure = parsed,
            "mindist" => data.mindist = parsed,
            "poisonpower" => data.poison_power = parsed,
            "poisontype" => data.poison_type = parsed,
            "poisonchance" => data.poison_chance = parsed,
            "drinkspecial" => data.drinkspecial = parsed,
            "drinkinvpots" => data.drink_inventory_potions = parsed,
            _ => unknown.push(UnknownSimpleBaddyArgument {
                name: name.to_string(),
                value: value.to_string(),
            }),
        }
        rest = next;
    }

    SimpleBaddyParseResult { data, unknown }
}
pub fn apply_simple_baddy_create_message(
    character: &mut Character,
    args: Option<&str>,
    current_tick: i32,
) -> Vec<UnknownSimpleBaddyArgument> {
    let mut data = match character.driver_state.take() {
        Some(CharacterDriverState::SimpleBaddy(data)) => data,
        Some(
            CharacterDriverState::Clara(_)
            | CharacterDriverState::TwoSkelly(_)
            | CharacterDriverState::Lab2Undead(_)
            | CharacterDriverState::Merchant(_)
            | CharacterDriverState::Aclerk(_)
            | CharacterDriverState::Lostcon(_)
            | CharacterDriverState::Bank(_)
            | CharacterDriverState::Trader(_)
            | CharacterDriverState::Janitor(_)
            | CharacterDriverState::GateWelcome(_)
            | CharacterDriverState::GateFight(_)
            | CharacterDriverState::Clanmaster(_)
            | CharacterDriverState::ClanFound(_)
            | CharacterDriverState::Clanclerk(_)
            | CharacterDriverState::Clubmaster(_)
            | CharacterDriverState::MilitaryMaster(_)
            | CharacterDriverState::MilitaryAdvisor(_)
            | CharacterDriverState::ArenaMaster(_)
            | CharacterDriverState::ArenaFighter(_)
            | CharacterDriverState::ArenaManager(_)
            | CharacterDriverState::Dungeonmaster(_)
            | CharacterDriverState::Dungeonfighter(_)
            | CharacterDriverState::Macro(_)
            | CharacterDriverState::Camhermit(_)
            | CharacterDriverState::Yoakin(_)
            | CharacterDriverState::Terion(_)
            | CharacterDriverState::Gwendylon(_)
            | CharacterDriverState::Greeter(_)
            | CharacterDriverState::Jessica(_)
            | CharacterDriverState::Jiu(_)
            | CharacterDriverState::ForestRanger(_)
            | CharacterDriverState::Brithildie(_)
            | CharacterDriverState::Nook(_)
            | CharacterDriverState::Lydia(_)
            | CharacterDriverState::Robber(_)
            | CharacterDriverState::Sanoa(_)
            | CharacterDriverState::Asturin(_)
            | CharacterDriverState::Reskin(_)
            | CharacterDriverState::Guiwynn(_)
            | CharacterDriverState::James(_)
            | CharacterDriverState::Balltrap(_)
            | CharacterDriverState::Logain(_)
            | CharacterDriverState::Superior(_)
            | CharacterDriverState::Moonie(_)
            | CharacterDriverState::Vampire(_)
            | CharacterDriverState::Vampire2(_)
            | CharacterDriverState::Astro1(_)
            | CharacterDriverState::Astro2(_)
            | CharacterDriverState::Thomas(_)
            | CharacterDriverState::SirJones(_)
            | CharacterDriverState::Seymour(_)
            | CharacterDriverState::Kelly(_)
            | CharacterDriverState::Lampghost(_)
            | CharacterDriverState::Carlos(_)
            | CharacterDriverState::Kassim(_)
            | CharacterDriverState::Supermax(_)
            | CharacterDriverState::Tester(_)
            | CharacterDriverState::Engrave(_)
            | CharacterDriverState::FdemonArmy(_)
            | CharacterDriverState::Islena(_)
            | CharacterDriverState::PalaceGuard(_)
            | CharacterDriverState::GolemKeyhold(_)
            | CharacterDriverState::ForestImp(_)
            | CharacterDriverState::ForestWilliam(_)
            | CharacterDriverState::ForestHermit(_)
            | CharacterDriverState::TwoSanwyn(_)
            | CharacterDriverState::TwoAlchemist(_)
            | CharacterDriverState::TwoBarkeeper(_)
            | CharacterDriverState::TwoServant(_)
            | CharacterDriverState::TwoGuard(_)
            | CharacterDriverState::TwoThiefGuard(_)
            | CharacterDriverState::TwoThiefMaster(_)
            | CharacterDriverState::Nomad(_)
            | CharacterDriverState::Madhermit(_)
            | CharacterDriverState::LqNpc(_)
            | CharacterDriverState::LabGnome(_)
            | CharacterDriverState::Lab2Herald(_)
            | CharacterDriverState::Lab2Deamon(_)
            | CharacterDriverState::Lab3Passguard(_)
            | CharacterDriverState::Lab3Prisoner(_)
            | CharacterDriverState::Lab4Seyan(_)
            | CharacterDriverState::Lab4Gnalb(_)
            | CharacterDriverState::Lab5Seyan(_)
            | CharacterDriverState::Lab5Daemon(_)
            | CharacterDriverState::Lab5Mage(_)
            | CharacterDriverState::StrategyWorker(_)
            | CharacterDriverState::WarpFighter(_)
            | CharacterDriverState::Warpmaster(_)
            | CharacterDriverState::SmuggleCom(_)
            | CharacterDriverState::Rouven(_)
            | CharacterDriverState::Aristocrat(_)
            | CharacterDriverState::Yoatin(_)
            | CharacterDriverState::SpiritBran(_)
            | CharacterDriverState::GuardBran(_)
            | CharacterDriverState::BrennethBran(_)
            | CharacterDriverState::Broklin(_)
            | CharacterDriverState::CountBran(_)
            | CharacterDriverState::CountessaBran(_)
            | CharacterDriverState::DaughterBran(_)
            | CharacterDriverState::ForestBran(_)
            | CharacterDriverState::Grinnich(_)
            | CharacterDriverState::Shanra(_)
            | CharacterDriverState::DwarfChief(_)
            | CharacterDriverState::LostDwarf(_)
            | CharacterDriverState::DwarfShaman(_)
            | CharacterDriverState::DwarfSmith(_)
            | CharacterDriverState::MissionGiver(_)
            | CharacterDriverState::Gorwin(_)
            | CharacterDriverState::TeufelGambler(_)
            | CharacterDriverState::TeufelQuest(_)
            | CharacterDriverState::Nop(_)
            | CharacterDriverState::Rammy(_)
            | CharacterDriverState::Jaz(_)
            | CharacterDriverState::Fiona(_)
            | CharacterDriverState::BridgeGuard(_)
            | CharacterDriverState::Gladiator(_)
            | CharacterDriverState::Ramin(_)
            | CharacterDriverState::Arkhatamonk(_)
            | CharacterDriverState::Captain(_)
            | CharacterDriverState::Judge(_)
            | CharacterDriverState::Jada(_)
            | CharacterDriverState::Potmaker(_)
            | CharacterDriverState::Hunter(_)
            | CharacterDriverState::Thaipan(_)
            | CharacterDriverState::Trainer(_)
            | CharacterDriverState::Kidnappee(_)
            | CharacterDriverState::Clerk(_)
            | CharacterDriverState::Krenach(_)
            | CharacterDriverState::Professor(_),
        ) => SimpleBaddyDriverData::default(),
        None => SimpleBaddyDriverData::default(),
    };

    let unknown = if let Some(args) = args.filter(|args| !args.is_empty()) {
        let parsed = parse_simple_baddy_driver_args(args);
        data = parsed.data;
        parsed.unknown
    } else {
        Vec::new()
    };

    data.creation_time = current_tick;
    // C `fight_driver_set_dist(cn, dat->startdist, dat->chardist,
    // dat->stopdist)` (`simple_baddy.c:189`): seeds the independent
    // `DRD_FIGHTDRIVER` slot's distance config from simple_baddy's own
    // freshly (re)parsed copy, leaving any already-tracked enemies/home
    // position/last-hit tick untouched (`fight_driver_set_dist` itself
    // only ever writes the three distance fields).
    let fight_driver = character
        .fight_driver
        .get_or_insert_with(FightDriverData::default);
    fight_driver.start_dist = data.startdist;
    fight_driver.char_dist = data.chardist;
    fight_driver.stop_dist = data.stopdist;
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(data));
    character
        .driver_messages
        .retain(|message| message.message_type != NT_CREATE);

    if character.inventory.get(30).and_then(|slot| *slot).is_some()
        && character.flags.contains(CharacterFlags::NOBODY)
    {
        character.flags.remove(CharacterFlags::NOBODY);
        character.flags.insert(CharacterFlags::ITEMDEATH);
    }

    unknown
}
pub fn process_simple_baddy_messages(
    character: &mut Character,
    carried_items: &[Item],
) -> Vec<SimpleBaddyMessageOutcome> {
    let drink_inventory_potions = matches!(
        character.driver_state.as_ref(),
        Some(CharacterDriverState::SimpleBaddy(data)) if data.drink_inventory_potions != 0
    );
    let helper = match character.driver_state.as_ref() {
        Some(CharacterDriverState::SimpleBaddy(data)) => data.helper,
        _ => 0,
    };
    let aggressive = match character.driver_state.as_ref() {
        Some(CharacterDriverState::SimpleBaddy(data)) => data.aggressive,
        _ => 0,
    };
    let poison = match character.driver_state.as_ref() {
        Some(CharacterDriverState::SimpleBaddy(data)) if data.poison_power > 0 => Some((
            data.poison_power as u16,
            data.poison_type.max(0) as u16,
            data.poison_chance,
        )),
        _ => None,
    };
    let helpid = match character.driver_state.as_ref() {
        Some(CharacterDriverState::SimpleBaddy(data)) => data.helpid,
        _ => 0,
    };
    let mut outcomes = Vec::new();

    let messages = std::mem::take(&mut character.driver_messages);
    for message in messages {
        if message.message_type == NT_CHAR && helper != 0 && message.dat1 > 0 {
            outcomes.push(SimpleBaddyMessageOutcome::BlessFriend {
                target_id: crate::ids::CharacterId(message.dat1 as u32),
            });
        }

        if message.message_type == NT_CHAR && aggressive != 0 && message.dat1 > 0 {
            outcomes.push(SimpleBaddyMessageOutcome::StandardAggro {
                target_id: CharacterId(message.dat1 as u32),
                priority: 0,
                require_visible: true,
                hurtme: false,
            });
        }

        if message.message_type == NT_SEEHIT && helper != 0 && message.dat1 > 0 && message.dat2 > 0
        {
            outcomes.push(SimpleBaddyMessageOutcome::StandardSeenHit {
                attacker_id: CharacterId(message.dat1 as u32),
                victim_id: CharacterId(message.dat2 as u32),
            });
        }

        if message.message_type == NT_TEXT && message.dat3 > 0 {
            outcomes.push(SimpleBaddyMessageOutcome::TextNotification {
                speaker_id: CharacterId(message.dat3 as u32),
                text_token: message.dat2,
                text: message.text.clone(),
            });
        }

        if message.message_type == NT_GOTHIT && drink_inventory_potions {
            if let Some(item_id) = find_simple_baddy_inventory_potion(
                character,
                carried_items,
                CharacterValue::Hp,
                2,
                PotionUseReason::LowHp,
            ) {
                outcomes.push(SimpleBaddyMessageOutcome::UseInventoryPotion {
                    item_id,
                    reason: PotionUseReason::LowHp,
                });
            }

            if let Some(item_id) = find_simple_baddy_inventory_potion(
                character,
                carried_items,
                CharacterValue::Mana,
                4,
                PotionUseReason::LowMana,
            ) {
                outcomes.push(SimpleBaddyMessageOutcome::UseInventoryPotion {
                    item_id,
                    reason: PotionUseReason::LowMana,
                });
            }
        }

        if message.message_type == NT_GOTHIT {
            outcomes.push(SimpleBaddyMessageOutcome::NoteHit);
        }

        if message.message_type == NT_GOTHIT && message.dat1 > 0 {
            outcomes.push(SimpleBaddyMessageOutcome::StandardAggro {
                target_id: CharacterId(message.dat1 as u32),
                priority: 1,
                require_visible: false,
                hurtme: true,
            });
        }

        if message.message_type == NT_DIDHIT && message.dat1 > 0 && message.dat2 > 0 {
            if let Some((power, poison_type, chance)) = poison {
                outcomes.push(SimpleBaddyMessageOutcome::PoisonHit {
                    target_id: CharacterId(message.dat1 as u32),
                    power,
                    poison_type,
                    chance,
                });
            }
        }

        if message.message_type == NT_NPC
            && helpid != 0
            && message.dat1 == helpid
            && message.dat2 > 0
        {
            outcomes.push(SimpleBaddyMessageOutcome::AddEnemy {
                caller_id: CharacterId(message.dat2 as u32),
                target_id: CharacterId(message.dat3.max(0) as u32),
            });
        }

        if message.message_type == NT_DEAD && message.dat1 > 0 {
            outcomes.push(SimpleBaddyMessageOutcome::RemoveEnemy {
                target_id: CharacterId(message.dat1 as u32),
            });
        }
    }

    outcomes
}
pub fn add_simple_baddy_enemy(
    character: &mut Character,
    caller: &Character,
    target_id: CharacterId,
    current_tick: i32,
) -> bool {
    if caller.id == character.id || caller.group != character.group {
        return false;
    }

    add_simple_baddy_enemy_unchecked(character, target_id, 1, current_tick)
}
pub fn add_simple_baddy_enemy_unchecked(
    character: &mut Character,
    target_id: CharacterId,
    priority: i32,
    current_tick: i32,
) -> bool {
    // C `fight_driver_add_enemy` (`drvlib.c:2056`) reads/writes the
    // `DRD_FIGHTDRIVER` slot independently of whatever driver `cn` is
    // currently running - it is shared by the `CDR_SIMPLEBADDY`/
    // `CDR_DUNGEONFIGHTER` NPC driver, the `CDR_LOSTCON` self-defense
    // driver, and (via the player-side `no*` toggles) a normal playing
    // character. No `driver_state` gate here, matching that.
    let data = character
        .fight_driver
        .get_or_insert_with(FightDriverData::default);

    if let Some(enemy) = data
        .enemies
        .iter_mut()
        .take(9)
        .find(|enemy| enemy.target_id == target_id)
    {
        enemy.priority = priority;
        enemy.last_seen_tick = current_tick;
        return false;
    }

    let enemy = SimpleBaddyEnemy {
        target_id,
        priority,
        last_seen_tick: current_tick,
        visible: false,
        last_x: 0,
        last_y: 0,
    };
    if data.enemies.len() < 10 {
        data.enemies.push(enemy);
    } else {
        data.enemies[9] = enemy;
    }
    true
}
pub fn remove_simple_baddy_enemy(character: &mut Character, target_id: CharacterId) -> bool {
    // C `fight_driver_remove_enemy` (`drvlib.c:2144`): same
    // driver-independent `DRD_FIGHTDRIVER` slot as `add_simple_baddy_enemy_
    // unchecked` above - no `driver_state` gate.
    let Some(data) = character.fight_driver.as_mut() else {
        return false;
    };

    let previous_len = data.enemies.len();
    data.enemies.retain(|enemy| enemy.target_id != target_id);
    data.enemies.len() != previous_len
}
fn find_simple_baddy_inventory_potion(
    character: &Character,
    carried_items: &[Item],
    value: CharacterValue,
    divisor: i32,
    reason: PotionUseReason,
) -> Option<ItemId> {
    let max_value = character_value(character, value);
    if max_value == 0 {
        return None;
    }

    let current = match value {
        CharacterValue::Hp => character.hp,
        CharacterValue::Mana => character.mana,
        _ => return None,
    };
    if current >= max_value * POWERSCALE / divisor {
        return None;
    }

    character
        .inventory
        .get(30..INVENTORY_SIZE)
        .unwrap_or_default()
        .iter()
        .flatten()
        .find(|item_id| {
            carried_items
                .iter()
                .find(|item| item.id == **item_id)
                .is_some_and(|item| {
                    item.driver == IDR_POTION
                        && match reason {
                            PotionUseReason::LowHp => drdata(item, 1) != 0,
                            PotionUseReason::LowMana => drdata(item, 2) != 0,
                        }
                })
        })
        .copied()
}
fn character_value(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .get(1)
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default()
        .into()
}
fn drdata(item: &Item, index: usize) -> u8 {
    item.driver_data.get(index).copied().unwrap_or_default()
}
pub(crate) fn next_legacy_name_value(input: &str) -> Option<(&str, &str, &str)> {
    let input = input.trim_start_matches(char::is_whitespace);
    let name_len = input
        .bytes()
        .take(60)
        .take_while(|byte| byte.is_ascii_alphabetic())
        .count();
    if name_len == 0 {
        return None;
    }
    let name = &input[..name_len];
    let input = input[name_len..].trim_start_matches(char::is_whitespace);
    let input = input.strip_prefix('=')?;
    let input = input.trim_start_matches(char::is_whitespace);
    let value_len = input
        .bytes()
        .take(60)
        .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
        .count();
    let value = &input[..value_len];
    let input = input[value_len..].strip_prefix(';')?;
    Some((name, value, input.trim_start_matches(char::is_whitespace)))
}
