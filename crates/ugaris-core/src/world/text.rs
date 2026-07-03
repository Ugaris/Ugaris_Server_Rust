use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldSoundSpecial {
    pub character_id: CharacterId,
    pub special: AreaSoundSpecial,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSystemText {
    pub character_id: CharacterId,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldAreaText {
    pub x: u16,
    pub y: u16,
    pub max_distance: u16,
    pub message: String,
}

impl World {
    pub fn notify_twocity_pick_from_character(&mut self, character_id: CharacterId) {
        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        let x = character.x;
        let y = character.y;
        self.notify_area(x, y, NT_NPC, NTID_TWOCITY_PICK, character_id.0 as i32, 0);
    }

    pub fn notify_area(
        &mut self,
        x: u16,
        y: u16,
        message_type: i32,
        dat1: i32,
        dat2: i32,
        dat3: i32,
    ) {
        let min_x = x.saturating_sub(16);
        let max_x = x.saturating_add(16);
        let min_y = y.saturating_sub(16);
        let max_y = y.saturating_add(16);
        for character in self.characters.values_mut() {
            if character.x >= min_x
                && character.x <= max_x
                && character.y >= min_y
                && character.y <= max_y
            {
                character.push_driver_message(message_type, dat1, dat2, dat3);
            }
        }
    }

    pub fn sound_area_specials(
        &self,
        x: usize,
        y: usize,
        sound_type: u32,
    ) -> Vec<WorldSoundSpecial> {
        let min_x = x.saturating_sub(16);
        let max_x = x.saturating_add(16).min(self.map.width().saturating_sub(1));
        let min_y = y.saturating_sub(16);
        let max_y = y
            .saturating_add(16)
            .min(self.map.height().saturating_sub(1));
        let sectors = (sound_type == u32::from(LOG_TALK)).then(|| SoundSectors::build(&self.map));

        let mut specials = Vec::new();
        for character in self.characters.values() {
            if !character
                .flags
                .contains(CharacterFlags::USED | CharacterFlags::PLAYER)
            {
                continue;
            }
            let character_x = usize::from(character.x);
            let character_y = usize::from(character.y);
            if character_x < min_x
                || character_x > max_x
                || character_y < min_y
                || character_y > max_y
            {
                continue;
            }
            if sectors.as_ref().is_some_and(|sectors| {
                !sectors.sector_hear(&self.map, x, y, character_x, character_y)
            }) {
                continue;
            }

            let dist_x = i32::from(character.x) - x as i32;
            let dist_y = i32::from(character.y) - y as i32;
            let dist = (dist_x * dist_x + dist_y * dist_y) * 10;
            specials.push(WorldSoundSpecial {
                character_id: character.id,
                special: AreaSoundSpecial {
                    special_type: sound_type,
                    opt1: -dist,
                    opt2: dist_x * 100,
                },
            });
        }
        specials
    }

    pub fn queue_sound_area(&mut self, x: usize, y: usize, sound_type: u32) {
        let specials = self.sound_area_specials(x, y, sound_type);
        self.pending_sound_specials.extend(specials);
    }

    pub fn drain_pending_sound_specials(&mut self) -> Vec<WorldSoundSpecial> {
        self.pending_sound_specials.drain(..).collect()
    }

    pub fn queue_system_text(&mut self, character_id: CharacterId, message: impl Into<String>) {
        self.pending_system_texts.push(WorldSystemText {
            character_id,
            message: message.into(),
        });
    }

    pub fn drain_pending_system_texts(&mut self) -> Vec<WorldSystemText> {
        self.pending_system_texts.drain(..).collect()
    }

    pub fn drain_pending_area_texts(&mut self) -> Vec<WorldAreaText> {
        self.pending_area_texts.drain(..).collect()
    }

    pub(crate) fn apply_tabunga_text_notification(
        &mut self,
        character_id: CharacterId,
        speaker_id: CharacterId,
        text: &str,
    ) -> bool {
        if !text.to_ascii_lowercase().contains("tabunga") {
            return false;
        }
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let Some(speaker) = self.characters.get(&speaker_id) else {
            return false;
        };
        if !speaker.flags.contains(CharacterFlags::GOD) || char_dist(&character, speaker) >= 3 {
            return false;
        }

        for message in tabunga_lines(&character) {
            self.pending_area_texts.push(WorldAreaText {
                x: character.x,
                y: character.y,
                max_distance: SAY_DIST as u16,
                message,
            });
        }
        true
    }
}

pub(crate) fn tabunga_lines(character: &Character) -> Vec<String> {
    let present = |value| character_value_present(character, value);
    let base = |value| character_value_base(character, value);
    vec![
        format!("{} ({}):", character.name, character.level),
        format!(
            "HP:        {:3}/{:3} ({})",
            present(CharacterValue::Hp),
            base(CharacterValue::Hp),
            character.hp / POWERSCALE
        ),
        format!(
            "Endurance: {:3}/{:3} ({})",
            present(CharacterValue::Endurance),
            base(CharacterValue::Endurance),
            character.endurance / POWERSCALE
        ),
        format!(
            "Mana:      {:3}/{:3} ({})",
            present(CharacterValue::Mana),
            base(CharacterValue::Mana),
            character.mana / POWERSCALE
        ),
        format!(
            "Wisdom:    {:3}/{:3}",
            present(CharacterValue::Wisdom),
            base(CharacterValue::Wisdom)
        ),
        format!(
            "Intuition: {:3}/{:3}",
            present(CharacterValue::Intelligence),
            base(CharacterValue::Intelligence)
        ),
        format!(
            "Agility:   {:3}/{:3}",
            present(CharacterValue::Agility),
            base(CharacterValue::Agility)
        ),
        format!(
            "Strength:  {:3}/{:3}",
            present(CharacterValue::Strength),
            base(CharacterValue::Strength)
        ),
        format!(
            "Hand2Hand: {:3}/{:3}",
            present(CharacterValue::Hand),
            base(CharacterValue::Hand)
        ),
        format!(
            "Sword:     {:3}/{:3}",
            present(CharacterValue::Sword),
            base(CharacterValue::Sword)
        ),
        format!(
            "Twohanded: {:3}/{:3}",
            present(CharacterValue::TwoHand),
            base(CharacterValue::TwoHand)
        ),
        format!(
            "Attack:    {:3}/{:3}",
            present(CharacterValue::Attack),
            base(CharacterValue::Attack)
        ),
        format!(
            "Parry:     {:3}/{:3}",
            present(CharacterValue::Parry),
            base(CharacterValue::Parry)
        ),
        format!(
            "Tactics:   {:3}/{:3}",
            present(CharacterValue::Tactics),
            base(CharacterValue::Tactics)
        ),
        format!(
            "Immunity:  {:3}/{:3}",
            present(CharacterValue::Immunity),
            base(CharacterValue::Immunity)
        ),
        format!(
            "Bless:     {:3}/{:3}",
            present(CharacterValue::Bless),
            base(CharacterValue::Bless)
        ),
        format!(
            "M-Shield:  {:3}/{:3}  ({})",
            present(CharacterValue::MagicShield),
            base(CharacterValue::MagicShield),
            character.lifeshield / POWERSCALE
        ),
        format!(
            "Flash:     {:3}/{:3}",
            present(CharacterValue::Flash),
            base(CharacterValue::Flash)
        ),
        format!(
            "Freeze:    {:3}/{:3}",
            present(CharacterValue::Freeze),
            base(CharacterValue::Freeze)
        ),
        format!(
            "Speed:     {:3}/{:3}",
            present(CharacterValue::Speed),
            base(CharacterValue::Speed)
        ),
        format!(
            "F-Ball:    {:3}/{:3}",
            present(CharacterValue::Fireball),
            base(CharacterValue::Fireball)
        ),
        format!(
            "Percept:   {:3}/{:3}",
            present(CharacterValue::Percept),
            base(CharacterValue::Percept)
        ),
        format!(
            "Stealth:   {:3}/{:3}",
            present(CharacterValue::Stealth),
            base(CharacterValue::Stealth)
        ),
        format!(
            "Warcry:    {:3}/{:3}",
            present(CharacterValue::Warcry),
            base(CharacterValue::Warcry)
        ),
        format!(
            "P_DEMON:   {:3}",
            character_profession(character, profession::DEMON)
        ),
        format!(
            "P_CLAN:    {:3}",
            character_profession(character, profession::CLAN)
        ),
        format!(
            "P_LIGHT:   {:3}",
            character_profession(character, profession::LIGHT)
        ),
        format!(
            "P_DARK:    {:3}",
            character_profession(character, profession::DARK)
        ),
        format!(
            "Offensive Value: {}, WV: {}",
            attack_skill(
                base(CharacterValue::Attack) > 0,
                base(CharacterValue::Sword)
                    .max(base(CharacterValue::Hand))
                    .max(base(CharacterValue::TwoHand)),
                base(CharacterValue::Attack),
                base(CharacterValue::Tactics),
                character_value(character, CharacterValue::Rage),
                character.flags.contains(CharacterFlags::EDEMON),
                character.level as i32,
                spell_average(
                    base(CharacterValue::Bless),
                    base(CharacterValue::Heal),
                    base(CharacterValue::Freeze),
                    base(CharacterValue::MagicShield),
                    base(CharacterValue::Flash),
                    base(CharacterValue::Fireball),
                    base(CharacterValue::Pulse),
                ),
            ),
            base(CharacterValue::Weapon)
        ),
        format!(
            "Defensive Value: {}, AV: {}",
            base(CharacterValue::Parry),
            base(CharacterValue::Armor) / 20
        ),
        format!(
            "x={}, y={}, speedmode={}",
            character.rest_x, character.rest_y, character.speed_mode as u8
        ),
        format!(
            "undead={}, alive={}",
            if character.flags.contains(CharacterFlags::UNDEAD) {
                "yes"
            } else {
                "no"
            },
            if character.flags.contains(CharacterFlags::ALIVE) {
                "yes"
            } else {
                "no"
            }
        ),
    ]
}
