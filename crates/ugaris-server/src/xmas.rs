use super::*;

pub(crate) fn apply_xmasmaker(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
) -> bool {
    grant_template_item_smart(world, loader, character_id, "xmaspop").is_some()
}

pub(crate) const XMAS_TREE_GIFT_TEMPLATES: [&str; 17] = [
    "ad_bracelet1",
    "ad_bracelet2",
    "ad_ring1",
    "ad_ring2",
    "ad_ring3",
    "ad_ring4",
    "ad_ring5",
    "ad_necklace1",
    "ad_necklace2",
    "ad_cape1",
    "ad_cape2",
    "ad_cape3",
    "ad_boots1",
    "ad_boots2",
    "ad_boots3",
    "ad_belt1",
    "ad_belt2",
];

pub(crate) const XMAS_TREE_GIFT_GODS: [&str; 3] = ["Eddow", "Freya", "Sauron"];

pub(crate) const XMAS_MAX_SKILLS: usize = 3;

pub(crate) const XMAS_MAX_SKILL_VALUE: i16 = 20;

pub(crate) const XMAS_SPECIAL_MAX_VALUE: i16 = 20;

pub(crate) const XMAS_ENHANCE_SKILLS: [CharacterValue; 35] = [
    CharacterValue::Hp,
    CharacterValue::Endurance,
    CharacterValue::Mana,
    CharacterValue::Wisdom,
    CharacterValue::Intelligence,
    CharacterValue::Agility,
    CharacterValue::Strength,
    CharacterValue::Light,
    CharacterValue::Speed,
    CharacterValue::Pulse,
    CharacterValue::Dagger,
    CharacterValue::Hand,
    CharacterValue::Staff,
    CharacterValue::Sword,
    CharacterValue::TwoHand,
    CharacterValue::Attack,
    CharacterValue::Parry,
    CharacterValue::Warcry,
    CharacterValue::Tactics,
    CharacterValue::Surround,
    CharacterValue::BodyControl,
    CharacterValue::SpeedSkill,
    CharacterValue::Barter,
    CharacterValue::Percept,
    CharacterValue::Stealth,
    CharacterValue::Bless,
    CharacterValue::Heal,
    CharacterValue::Freeze,
    CharacterValue::MagicShield,
    CharacterValue::Flash,
    CharacterValue::Fireball,
    CharacterValue::Regenerate,
    CharacterValue::Meditate,
    CharacterValue::Immunity,
    CharacterValue::Duration,
];

#[derive(Debug, Clone)]
pub(crate) struct XmasTreeRng {
    state: u64,
}

impl XmasTreeRng {
    pub(crate) fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9e37_79b9_7f4a_7c15,
        }
    }

    pub(crate) fn random(&mut self, limit: usize) -> usize {
        if limit == 0 {
            return 0;
        }
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.state >> 32) as usize) % limit
    }
}

pub(crate) fn random_xmas_skill_value(rng: &mut XmasTreeRng) -> i16 {
    let mut value = rng.random((XMAS_MAX_SKILL_VALUE / 2 + 1) as usize) as i16;
    if rng.random(100) < 30 {
        value += rng.random((XMAS_MAX_SKILL_VALUE / 4) as usize) as i16;
    }
    if rng.random(100) < 10 {
        value += rng.random((XMAS_MAX_SKILL_VALUE / 4) as usize) as i16;
    }
    value.min(XMAS_MAX_SKILL_VALUE)
}

pub(crate) fn random_xmas_special_value(rng: &mut XmasTreeRng) -> i16 {
    let mut value = rng.random((XMAS_SPECIAL_MAX_VALUE / 2 + 1) as usize) as i16;
    if rng.random(100) < 20 {
        value += rng.random((XMAS_SPECIAL_MAX_VALUE / 4) as usize) as i16;
    }
    if rng.random(100) < 10 {
        value += rng.random((XMAS_SPECIAL_MAX_VALUE / 4) as usize) as i16;
    }
    if rng.random(100) < 5 {
        value = XMAS_SPECIAL_MAX_VALUE;
    }
    value.min(XMAS_SPECIAL_MAX_VALUE)
}

pub(crate) fn grant_xmas_tree_gift(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    seed: u64,
) -> Option<String> {
    let mut rng = XmasTreeRng::new(seed);
    let template = XMAS_TREE_GIFT_TEMPLATES[(seed as usize) % XMAS_TREE_GIFT_TEMPLATES.len()];
    let recipient_name = world.characters.get(&character_id)?.name.clone();
    let mut item = loader
        .instantiate_item_template(template, Some(character_id))
        .ok()?;
    enhance_xmas_item(&mut item, &mut rng);
    let god = XMAS_TREE_GIFT_GODS[rng.random(XMAS_TREE_GIFT_GODS.len())];
    item.description =
        format!("To {recipient_name}, with holiday blessings from {god}.\nMerry Christmas!");
    let item_name = item.name.clone();
    let (give_result, drop_x, drop_y) = {
        let character = world.characters.get_mut(&character_id)?;
        let result = give_item_to_character(character, &mut item, GiveItemFlags::ALLOW_DROP);
        (result, usize::from(character.x), usize::from(character.y))
    };
    match give_result {
        GiveItemResult::Ok => {}
        GiveItemResult::Dropped => {
            if !world.map.drop_item_extended(&mut item, drop_x, drop_y, 1) {
                return None;
            }
        }
        GiveItemResult::Money => {}
        GiveItemResult::Full | GiveItemResult::Failed => return None,
    }
    world.add_item(item);
    Some(item_name)
}

pub(crate) fn xmas_event_from_ymd(year: i32, month: u32, day: u32) -> (bool, i32) {
    if month == 12 && day >= 20 {
        (true, year)
    } else if month == 1 && day <= 7 {
        (true, year - 1)
    } else {
        (false, year)
    }
}

pub(crate) fn civil_from_unix_seconds(seconds: u64) -> (i32, u32, u32) {
    let days = (seconds / 86_400) as i64;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

pub(crate) fn current_xmas_event() -> (bool, i32) {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let (year, month, day) = civil_from_unix_seconds(seconds);
    xmas_event_from_ymd(year, month, day)
}

pub(crate) fn runtime_effective_xmas_event(runtime: &ServerRuntime) -> (bool, i32) {
    let (date_active, event_year) = current_xmas_event();
    match runtime.xmas_special_override {
        Some(flag) => (flag != 0, event_year),
        None => (date_active, event_year),
    }
}

pub(crate) fn runtime_effective_xmas_flag(runtime: &ServerRuntime) -> i32 {
    if runtime_effective_xmas_event(runtime).0 {
        1
    } else {
        0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum XmasTreeApplyResult {
    Dormant,
    AlreadyGranted,
    NeedsHolidayTreat,
    GiftGranted(String),
    NoSpace,
    MissingPlayer,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_xmastree(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: &mut PlayerRuntime,
    character_id: CharacterId,
    area_id: u16,
    is_xmas: bool,
    event_year: i32,
    gift_seed: u64,
) -> XmasTreeApplyResult {
    let has_holiday_treat = world
        .characters
        .get(&character_id)
        .and_then(|character| character.cursor_item)
        .and_then(|item_id| world.items.get(&item_id))
        .is_some_and(|item| item.driver == IDR_FOOD && item.driver_data.first() == Some(&3));

    match player.touch_xmas_tree(area_id, event_year, is_xmas, has_holiday_treat) {
        XmasTreeResult::Dormant => XmasTreeApplyResult::Dormant,
        XmasTreeResult::AlreadyGranted => XmasTreeApplyResult::AlreadyGranted,
        XmasTreeResult::NeedsHolidayTreat => XmasTreeApplyResult::NeedsHolidayTreat,
        XmasTreeResult::GiftGranted => {
            let Some(item_name) = grant_xmas_tree_gift(world, loader, character_id, gift_seed)
            else {
                player.unmark_xmas_tree(area_id);
                return XmasTreeApplyResult::NoSpace;
            };
            let Some(character) = world.characters.get_mut(&character_id) else {
                player.unmark_xmas_tree(area_id);
                return XmasTreeApplyResult::MissingPlayer;
            };
            if let Some(cursor_item_id) = character.cursor_item.take() {
                world.items.remove(&cursor_item_id);
                character.flags.insert(CharacterFlags::ITEMS);
            }
            XmasTreeApplyResult::GiftGranted(item_name)
        }
    }
}
