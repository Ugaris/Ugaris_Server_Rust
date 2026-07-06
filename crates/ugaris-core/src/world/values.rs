//! `/showvalues <name>` text command (C `command.c:8401-8409` ->
//! `show_values`, `command.c:521-537`, plus its `server_chat` body
//! `show_values_bg`, `src/system/tool.c:2940-3096`). Full-word
//! abbreviation - `cmdcmp(ptr, "showvalues", 4)`'s `minlen` is 4 (the
//! length of "show", not the full 10-letter word), so any prefix from
//! "show" up to "showvalues" matches - see `commands_admin.rs`'s
//! dispatch site for the `starts_with` abbreviation check this implies
//! (same idiom as the already-ported `/showattack`, `command.c:6397`,
//! `minlen` 6). No permission gate (any player can use it, unlike the
//! staff-only `/values`/`look_values`, which this module does not port -
//! see the "Cross-area transfer" `PORTING_TODO.md` task's Progress Log
//! for why it is a separate, larger remaining slice: it additionally
//! needs paying-player/PK/hardcore/playtime/bank-gold/mirror-area lines
//! this codebase has no equivalent of yet).
//!
//! `show_values` resolves the *argument* name (C `lookup_name`) and swaps
//! roles from there: `show_values_bg` sends the *caller's own*
//! class-specific abbreviated stat block to the resolved target, plus a
//! "Sent." confirmation logged to the caller - i.e. `/showvalues bob`
//! means "show my values to bob", not "show me bob's values" (compare
//! `show_values`'s `buf` construction, `coID` first then `ch[cn].ID`,
//! against `show_values_bg`'s parameter names after the `channel 1037`
//! swap in `chat.c:391-397`).
//!
//! C's real `show_values_bg` runs via a `server_chat` cross-area
//! broadcast: whichever area server has the target character loaded
//! replies. This codebase has no cross-process chat relay yet (see the
//! "Cross-area transfer" `PORTING_TODO.md` entry's gap (2)), so this is
//! the documented single-process-only slice: the target name is resolved
//! via `find_login_target` (C's synchronous `lookup_name`), and the
//! actual stat block is only delivered if the resolved character happens
//! to be loaded (online) in *this* process's `World` - matching every
//! other documented cross-area-chat gap in this codebase (e.g. `/tell` to
//! an offline/remote player).
use super::character_values::{character_value_base, character_value_present};
use super::lastseen::is_valid_lookup_name;
use super::npc_fight::simple_baddy_fight_skill;
use super::*;
use crate::attack::parry_skill;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowValuesRequest {
    pub caller_id: CharacterId,
    pub target_name: String,
}

impl World {
    /// C `/showvalues <name>`'s inline handler (`command.c:8401-8409`):
    /// trims leading whitespace, then calls `show_values(cn, ptr)` on the
    /// *entire* (untokenized) remainder - like `/look`, no alpha-only
    /// prefix extraction happens first. `show_values` itself
    /// (`command.c:521-537`) does no validation of its own beyond
    /// `lookup_name`'s own gate; an invalid shape (empty, non-alphabetic,
    /// or out-of-range length) resolves to C's `ID == -1` branch, "No
    /// player by that name." - the same text as a DB-confirmed-missing
    /// name, so both cases share one reply here (unlike `/look`'s
    /// distinct "Expected a character name." pre-check for an empty
    /// argument, which `show_values` has no equivalent of).
    pub fn queue_showvalues_command(&mut self, caller_id: CharacterId, target_name: &str) {
        let target_name = target_name.trim_start();
        if !is_valid_lookup_name(target_name) {
            self.queue_system_text(caller_id, "No player by that name.".to_string());
            return;
        }
        self.pending_showvalues_requests.push(ShowValuesRequest {
            caller_id,
            target_name: target_name.to_string(),
        });
    }

    pub fn drain_pending_showvalues_requests(&mut self) -> Vec<ShowValuesRequest> {
        self.pending_showvalues_requests.drain(..).collect()
    }
}

/// C `skill[].name` (`src/system/skill.c:26-84`) for exactly the value
/// indices `show_values_bg` displays - kept local (rather than reusing
/// the shared `CHARACTER_VALUE_NAMES` table) because that table's
/// wording diverges from the C skill array at an index this command
/// actually shows (`V_WARCRY`'s C name is "Warcry", one word;
/// `CHARACTER_VALUE_NAMES[20]` is "War Cry", two words) - this local
/// table is the letter-for-letter source of truth for this command.
fn skill_display_name(value: CharacterValue) -> &'static str {
    match value {
        CharacterValue::Hp => "Hitpoints",
        CharacterValue::Endurance => "Endurance",
        CharacterValue::Mana => "Mana",
        CharacterValue::Wisdom => "Wisdom",
        CharacterValue::Intelligence => "Intuition",
        CharacterValue::Agility => "Agility",
        CharacterValue::Strength => "Strength",
        CharacterValue::Dagger => "Dagger",
        CharacterValue::Hand => "Hand to Hand",
        CharacterValue::Staff => "Staff",
        CharacterValue::Sword => "Sword",
        CharacterValue::TwoHand => "Two-Handed",
        CharacterValue::ArmorSkill => "Armor Skill",
        CharacterValue::Attack => "Attack",
        CharacterValue::Parry => "Parry",
        CharacterValue::Warcry => "Warcry",
        CharacterValue::Tactics => "Tactics",
        CharacterValue::Surround => "Surround Hit",
        CharacterValue::BodyControl => "Body Control",
        CharacterValue::SpeedSkill => "Speed Skill",
        CharacterValue::Barter => "Bartering",
        CharacterValue::Percept => "Perception",
        CharacterValue::Stealth => "Stealth",
        CharacterValue::Bless => "Bless",
        CharacterValue::Heal => "Heal",
        CharacterValue::Freeze => "Freeze",
        CharacterValue::MagicShield => "Magic Shield",
        CharacterValue::Flash => "Lightning",
        CharacterValue::Fireball => "Fire",
        CharacterValue::Pulse => "Pulse",
        CharacterValue::Regenerate => "Regenerate",
        CharacterValue::Meditate => "Meditate",
        CharacterValue::Immunity => "Immunity",
        CharacterValue::Duration => "Duration",
        CharacterValue::Rage => "Rage",
        CharacterValue::Profession => "Profession",
        _ => "Unknown",
    }
}

/// `(present, base)` = C's `(value[1][n], value[0][n])` pair, matching
/// `show_values_bg`'s own `"%s: %d/%d"` argument order.
fn value_pair(character: &Character, value: CharacterValue) -> (i32, i32) {
    (
        character_value_present(character, value),
        character_value_base(character, value),
    )
}

/// One `"%s: %d/%d \010%s: %d/%d \020%s: %d/%d"` line (three columns,
/// C's literal backspace/DLE control-character separators reproduced
/// verbatim as `\u{8}`/`\u{10}`).
fn triple_line(
    character: &Character,
    a: CharacterValue,
    b: CharacterValue,
    c: CharacterValue,
) -> String {
    let (ap, ab) = value_pair(character, a);
    let (bp, bb) = value_pair(character, b);
    let (cp, cb) = value_pair(character, c);
    format!(
        "{}: {ap}/{ab} \u{8}{}: {bp}/{bb} \u{10}{}: {cp}/{cb}",
        skill_display_name(a),
        skill_display_name(b),
        skill_display_name(c),
    )
}

/// One `"%s: %d/%d \010%s: %d/%d"` line (two columns) - only the
/// Warrior branch's final line uses this shape (`tool.c:3051-3052`).
fn pair_line(character: &Character, a: CharacterValue, b: CharacterValue) -> String {
    let (ap, ab) = value_pair(character, a);
    let (bp, bb) = value_pair(character, b);
    format!(
        "{}: {ap}/{ab} \u{8}{}: {bp}/{bb}",
        skill_display_name(a),
        skill_display_name(b),
    )
}

/// C `show_values_bg`'s class-specific ability lines (`tool.c:2966-3089`),
/// selected by the caller's `CF_WARRIOR`/`CF_MAGE` flags exactly like C's
/// own `if/else if/else if` chain (a character with neither flag set - a
/// theoretical `CF_PLAYER`-less NPC - falls through to no lines at all,
/// matching C's silent no-`else` fallthrough).
fn class_value_lines(character: &Character) -> Vec<String> {
    use CharacterValue::*;
    let warrior = character.flags.contains(CharacterFlags::WARRIOR);
    let mage = character.flags.contains(CharacterFlags::MAGE);
    if warrior && mage {
        vec![
            triple_line(character, Hp, Endurance, Mana),
            triple_line(character, Wisdom, Intelligence, Agility),
            triple_line(character, Strength, Dagger, Hand),
            triple_line(character, Sword, TwoHand, ArmorSkill),
            triple_line(character, Attack, Parry, Warcry),
            triple_line(character, Tactics, Surround, BodyControl),
            triple_line(character, SpeedSkill, Bless, Heal),
            triple_line(character, Freeze, MagicShield, Flash),
            triple_line(character, Fireball, Pulse, Barter),
            triple_line(character, Percept, Stealth, Regenerate),
            triple_line(character, Meditate, Immunity, Profession),
        ]
    } else if warrior {
        vec![
            triple_line(character, Hp, Endurance, Wisdom),
            triple_line(character, Intelligence, Agility, Strength),
            triple_line(character, Dagger, Hand, Sword),
            triple_line(character, TwoHand, Rage, ArmorSkill),
            triple_line(character, Attack, Parry, Warcry),
            triple_line(character, Tactics, Surround, BodyControl),
            triple_line(character, SpeedSkill, Barter, Percept),
            triple_line(character, Stealth, Regenerate, Immunity),
            pair_line(character, Rage, Profession),
        ]
    } else if mage {
        vec![
            triple_line(character, Hp, Endurance, Mana),
            triple_line(character, Wisdom, Intelligence, Agility),
            triple_line(character, Strength, Dagger, Hand),
            triple_line(character, Staff, Bless, Heal),
            triple_line(character, Freeze, MagicShield, Flash),
            triple_line(character, Fireball, Pulse, Duration),
            triple_line(character, Barter, Percept, Stealth),
            triple_line(character, Meditate, Immunity, Profession),
        ]
    } else {
        Vec::new()
    }
}

/// C `show_values_bg` in full (`tool.c:2940-3096`), minus the
/// `getfirst_char`/`getnext_char` scan (the caller already has a resolved
/// `&Character`) and the "Sent." confirmation (queued separately by the
/// caller, since it goes to a different character id). Returns every
/// `tell_chat` line in order: header, class-specific ability lines,
/// then the shared armor/weapon/speed and offence/defence summary lines
/// (`tool.c:3092-3095`, computed via `get_fight_skill`/`get_attack_skill`/
/// `get_parry_skill` - reused here as `simple_baddy_fight_skill`/
/// `attack_skill`/`parry_skill`, the same primitives the NPC fight driver
/// already ports; live combat `rage` (`ch[].rage`, a separate field from
/// the "Rage" skill value) has no `Character` equivalent yet, so `0` is
/// passed like every other non-NPC caller of these primitives).
pub fn show_values_lines(character: &Character, items: &HashMap<ItemId, Item>) -> Vec<String> {
    let warrior = character.flags.contains(CharacterFlags::WARRIOR);
    let mage = character.flags.contains(CharacterFlags::MAGE);
    let class_name = if warrior && mage {
        "Seyan'Du"
    } else if warrior {
        "Warrior"
    } else {
        "Mage"
    };
    let arch = if character.flags.contains(CharacterFlags::ARCH) {
        "Arch-"
    } else {
        ""
    };
    let mut lines = vec![format!(
        "{}, Level {}, {arch}{class_name}",
        character.name, character.level
    )];
    lines.extend(class_value_lines(character));

    let armor = f64::from(character_value_base(character, CharacterValue::Armor)) / 20.0;
    lines.push(format!(
        "Armor: {armor:.2} \u{8}Weapon: {} \u{10}Speed: {}",
        character_value_base(character, CharacterValue::Weapon),
        character_value_base(character, CharacterValue::Speed),
    ));

    let fight_skill = simple_baddy_fight_skill(character, items);
    let spell_avg = spell_average(
        character_value_base(character, CharacterValue::Bless),
        character_value_base(character, CharacterValue::Heal),
        character_value_base(character, CharacterValue::Freeze),
        character_value_base(character, CharacterValue::MagicShield),
        character_value_base(character, CharacterValue::Flash),
        character_value_base(character, CharacterValue::Fireball),
        character_value_base(character, CharacterValue::Pulse),
    );
    let offence = attack_skill(
        character_value_present(character, CharacterValue::Attack) != 0,
        fight_skill,
        character_value_base(character, CharacterValue::Attack),
        character_value_base(character, CharacterValue::Tactics),
        0,
        character.flags.contains(CharacterFlags::EDEMON),
        character.level as i32,
        spell_avg,
    );
    let defence = parry_skill(
        character_value_present(character, CharacterValue::Parry) != 0,
        fight_skill,
        character_value_base(character, CharacterValue::Parry),
        character_value_base(character, CharacterValue::Tactics),
        0,
        character.flags.contains(CharacterFlags::EDEMON),
        character_value_present(character, CharacterValue::MagicShield) != 0,
        character_value_base(character, CharacterValue::MagicShield),
        spell_avg,
    );
    lines.push(format!("Offence: {offence} \u{8}Defence: {defence}"));
    lines
}
