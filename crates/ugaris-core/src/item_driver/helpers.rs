use super::*;

pub(crate) fn round_down_to_granularity(value: u32, granularity: u32) -> u32 {
    if granularity == 0 {
        value
    } else {
        (value / granularity) * granularity
    }
}

pub(crate) const BOOKCASE_RANDOM_TITLES: [&str; 26] = [
    "Tales of Two Towns by Karl Dicker",
    "The Art of Warfare by Hun Yu",
    "Chris Maas visits Carol by Karl Dicker",
    "Secrets of Adygalah Alchemy by Leonarda",
    "The rise and fall of the Seyan Empire by Takitus",
    "History of Ancient Astonia by Chiasmaphora",
    "Treatise on the Mastery of Mana by Mage Niuma",
    "The Song of the Warrior by Sir Regis Le Voleir",
    "The Book of Ishtar, Anonymous",
    "Concessions to Fear by Kentindher",
    "Poems of War and Homecoming by Melthold of Anten",
    "Memoires of a Lady-in-Waiting by Dame Sakanor",
    "Comprehension and Expression by Master Getsades",
    "Great Astonian Thinkers by Master Riotan",
    "A Portrait of the Seyan'Du as A Young Mage by Esjamocey",
    "Critique of Pure Courage by Imanel Dique",
    "Collected Essays by Lindmar the Elder",
    "The Reforming of Curves by Master Elyosod",
    "Advanced Agility in Forty-two Steps by Seyan'Du Bartoshi",
    "The Oath by Sheney",
    "The Strife for Light by Father Ignato",
    "The Aston Years by Lord Ironborn",
    "Luctim - Superstition or Reality? by Mintu the Enlightened",
    "I Have, Alas by Goytila",
    "A Midwinter Day's Wake by Pearshaks",
    "Fama Fraternitatis by Valentin Andreae",
];

pub(crate) const DEV_ID_DB: u32 = 0x01;

pub(crate) const DEV_ID_WARR: u32 = 0x06;

/// C `DEV_ID_RH` (`src/system/drdata.h:47`/`src/common/item_id.h:54`,
/// "ID of Roman Haas"): the area-1 hermit-quest teeth item below.
pub(crate) const DEV_ID_RH: u32 = 0x3A;

pub(crate) const fn make_item_id(dev_id: u32, nr: u32) -> u32 {
    (dev_id << 24) | nr
}

pub(crate) fn drdata_u16(item: &Item, idx: usize) -> u16 {
    let lo = u16::from(drdata(item, idx));
    let hi = u16::from(drdata(item, idx + 1));
    lo | (hi << 8)
}

pub(crate) fn drdata_u32(item: &Item, idx: usize) -> u32 {
    u32::from_le_bytes([
        drdata(item, idx),
        drdata(item, idx + 1),
        drdata(item, idx + 2),
        drdata(item, idx + 3),
    ])
}

pub(crate) fn set_drdata_u16(item: &mut Item, idx: usize, value: u16) {
    set_drdata(item, idx, value as u8);
    set_drdata(item, idx + 1, (value >> 8) as u8);
}

pub(crate) fn set_drdata_u32(item: &mut Item, idx: usize, value: u32) {
    for (offset, byte) in value.to_le_bytes().into_iter().enumerate() {
        set_drdata(item, idx + offset, byte);
    }
}

/// C `struct container`'s `owner`/`killer`/`access` ACL triad
/// (`container.h:25-28`), stored inline in a body-container `Item`'s own
/// `driver_data` blob (bytes 12-23, right after `create_dead_body_item`'s
/// existing 2-byte-aligned player-color fields at bytes 2-8) instead of
/// adding three generic fields to the crate-wide `Item` struct - matching
/// this codebase's established convention for driver-private per-item
/// state (`drdata_u32`/`set_drdata_u32` above, and every `item_driver/
/// area*.rs` module). Deliberately does not port `owner_not_seyan`
/// (`container.h:26`) - it only feeds a secondary "may bypass the
/// quest-item hold-shift restriction" nuance
/// (`src/system/do.c:1396-1399`) this codebase's container swap path
/// does not implement yet, not the core access-grant/deny gate below.
pub(crate) const GRAVE_OWNER_DRDATA_OFFSET: usize = 12;
pub(crate) const GRAVE_KILLER_DRDATA_OFFSET: usize = 16;
pub(crate) const GRAVE_ACCESS_DRDATA_OFFSET: usize = 20;

/// C `die_char`'s `con[ct].owner = charID(cn); con[ct].killer =
/// charID(co); con[ct].access = 0;` (`death.c:684-691`), run once at
/// body-container creation. `owner_id` is the character whose body this
/// is (may always access their own grave); `killer_id` is the character
/// who landed the killing blow (`None` for environmental deaths, C's
/// `co == 0` -> `charID(0) == 0`, matching `grave_access_denied`'s own
/// `killer == 0` no-op branch below).
pub(crate) fn set_grave_acl(
    item: &mut Item,
    owner_id: CharacterId,
    killer_id: Option<CharacterId>,
) {
    set_drdata_u32(item, GRAVE_OWNER_DRDATA_OFFSET, owner_id.0);
    set_drdata_u32(
        item,
        GRAVE_KILLER_DRDATA_OFFSET,
        killer_id.map_or(0, |id| id.0),
    );
    set_drdata_u32(item, GRAVE_ACCESS_DRDATA_OFFSET, 0);
}

pub(crate) fn grave_owner_id(item: &Item) -> u32 {
    drdata_u32(item, GRAVE_OWNER_DRDATA_OFFSET)
}

/// C `allow_body_db`'s `con[n].access = coID ? charID_ID(coID) : 0;`
/// (`death.c:1058`) - a grave only ever holds one grantable third-party
/// access slot, so granting to a new character silently overwrites any
/// previous grant, exactly like C's plain field assignment.
pub(crate) fn grant_grave_access(item: &mut Item, target_id: Option<CharacterId>) {
    set_drdata_u32(
        item,
        GRAVE_ACCESS_DRDATA_OFFSET,
        target_id.map_or(0, |id| id.0),
    );
}

/// C's grave-container access-control check, enforced identically at
/// every C call site (`act.c:1779-1781`, `do.c:1504-1506`,
/// `do.c:1381-1391`): `if (con[ct].owner && cn != owner && cn != killer
/// && cn != access) // access denied`. A container with no owner set
/// (every ordinary non-grave container - `owner` defaults to `0`, and
/// real character ids start at `1`) is never restricted.
pub(crate) fn grave_access_denied(item: &Item, character_id: CharacterId) -> bool {
    let owner = grave_owner_id(item);
    if owner == 0 || owner == character_id.0 {
        return false;
    }
    let killer = drdata_u32(item, GRAVE_KILLER_DRDATA_OFFSET);
    if killer != 0 && killer == character_id.0 {
        return false;
    }
    let access = drdata_u32(item, GRAVE_ACCESS_DRDATA_OFFSET);
    if access != 0 && access == character_id.0 {
        return false;
    }
    true
}

pub(crate) const EDEMON_SWITCH_COOLDOWN_TICKS: u64 = TICKS_PER_SECOND * 60 * 5;

/// C `level_value(level)` (`src/system/tool.c:1282`). Delegates to the
/// canonical `world::level_value` (`world/exp.rs`) - kept as a thin
/// `pub(crate)` re-export here since most `item_driver` modules already
/// `use super::*`/`use helpers::*` and importing `crate::world::level_value`
/// directly at every call site would be a larger, unrelated diff.
pub(crate) fn legacy_level_value(level: u32) -> u32 {
    crate::world::level_value(level)
}

pub(crate) fn check_item_requirements(character: &Character, item: &Item) -> bool {
    if character.level < u32::from(item.min_level) {
        return false;
    }
    if item.max_level != 0 && character.level > u32::from(item.max_level) {
        return false;
    }
    if item.needs_class & 1 != 0 && !character.flags.contains(CharacterFlags::WARRIOR) {
        return false;
    }
    if item.needs_class & 2 != 0 && !character.flags.contains(CharacterFlags::MAGE) {
        return false;
    }
    if item.needs_class & 4 != 0
        && !(character.flags.contains(CharacterFlags::WARRIOR)
            && character.flags.contains(CharacterFlags::MAGE))
    {
        return false;
    }
    if item.needs_class & 8 != 0 && !character.flags.contains(CharacterFlags::ARCH) {
        return false;
    }

    item.modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .all(|(&index, &required)| {
            if index >= 0 || required <= 0 {
                return true;
            }
            let value = (-index) as usize;
            character
                .values
                .get(1)
                .and_then(|values| values.get(value))
                .copied()
                .unwrap_or_default()
                >= required
        })
}

pub(crate) fn capped_resource(current: i32, added_units: u8, max_units: i32) -> i32 {
    (current + i32::from(added_units) * POWERSCALE).min(max_units * POWERSCALE)
}

pub(crate) fn max_value(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .first()
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default() as i32
}

pub(crate) fn drdata(item: &Item, idx: usize) -> u8 {
    item.driver_data.get(idx).copied().unwrap_or_default()
}

pub(crate) fn set_drdata(item: &mut Item, idx: usize, value: u8) {
    if item.driver_data.len() <= idx {
        item.driver_data.resize(idx + 1, 0);
    }
    item.driver_data[idx] = value;
}

pub(crate) fn write_drdata_u32(item: &mut Item, idx: usize, value: u32) {
    if item.driver_data.len() <= idx + 3 {
        item.driver_data.resize(idx + 4, 0);
    }
    item.driver_data[idx..idx + 4].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn clamp_legacy_coordinate(value: i32) -> u16 {
    value.clamp(0, i32::from(u16::MAX)) as u16
}
