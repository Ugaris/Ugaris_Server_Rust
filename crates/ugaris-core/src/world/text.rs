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

/// Byte-payload sibling of [`WorldSystemText`] for `log_char` calls whose C
/// text embeds a raw color-marker prefix (`COLOR_MARKER` = `\xb0`, e.g.
/// `COL_DARK_GRAY "Mission kill, %d to go."`, `military.c`'s
/// `check_military_solve` callers) that cannot round-trip through a Rust
/// `String` (`\xb0` alone is not valid UTF-8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSystemTextBytes {
    pub character_id: CharacterId,
    pub message: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldAreaText {
    pub x: u16,
    pub y: u16,
    pub max_distance: u16,
    pub message: String,
}

/// C `server_chat(channel, text)` (`src/system/chat/chat.c:827-834`): a
/// message fanned out to every connected player who has joined `channel`
/// (bit `1 << (channel - 1)` of `PlayerRuntime::chat_channels`), the same
/// delivery rule `apply_chat_command` uses for a player-authored channel
/// message. `message_bytes` is the fully-formed legacy wire payload
/// (10-digit zero sender-id field + color marker + text), matching C's
/// `"0000000000" COL_MAUVE "Grats: ..."` construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldChannelBroadcast {
    pub channel: u8,
    pub message_bytes: Vec<u8>,
}

/// C `prof[P_MAX]` (`src/system/prof.c`): profession display name and
/// `max` value (used by `prof_title`'s percent-of-max thresholds). Distinct
/// from `entity::PROFESSION_NAMES`, which mirrors the unrelated JSON export
/// table in `src/system/game/character.c` (that table spells index 7
/// "Master Trader" instead of prof.c's gameplay-visible "Trader").
pub const PROF_TABLE: [(&str, u8); 20] = [
    ("Athlete", 30),
    ("Alchemist", 50),
    ("Miner", 20),
    ("Assassin", 50),
    ("Thief", 30),
    ("Light Warrior", 30),
    ("Dark Warrior", 30),
    ("Trader", 20),
    ("Mercenary", 20),
    ("Clan Warrior", 30),
    ("Herbalist", 30),
    ("Demon", 30),
    ("empty", 30),
    ("empty", 30),
    ("empty", 30),
    ("empty", 30),
    ("empty", 30),
    ("empty", 30),
    ("empty", 30),
    ("empty", 30),
];

/// C `prof_title` (`src/system/prof.c`): percent-of-max skill title prefix.
fn profession_title(value: i16, max: u8) -> &'static str {
    let percent = 100 * i32::from(value) / i32::from(max.max(1));
    match percent {
        p if p < 15 => "a newbie ",
        p if p < 30 => "an apprentice ",
        p if p < 45 => "an intermediate ",
        p if p < 60 => "a fairly skilled ",
        p if p < 75 => "a skilled ",
        p if p < 90 => "a very skilled ",
        _ => "a master ",
    }
}

/// C `get_title` (`src/system/tool.c`): `CF_WON` title prefix.
pub fn look_character_title(target: &Character) -> &'static str {
    if !target.flags.contains(CharacterFlags::WON) {
        ""
    } else if target.flags.contains(CharacterFlags::FEMALE) {
        "Lady "
    } else {
        "Sir "
    }
}

/// C `Hename` (`src/system/tool.c`): capitalized he/she/it pronoun.
pub fn look_character_hename(target: &Character) -> &'static str {
    if target.flags.contains(CharacterFlags::MALE) {
        "He"
    } else if target.flags.contains(CharacterFlags::FEMALE) {
        "She"
    } else {
        "It"
    }
}

/// C `hisname` (`src/system/tool.c:1488`): lowercase his/her/its possessive
/// pronoun, gated the same way as `Hename`/`hename` (male/female/neuter).
pub fn hisname(character: &Character) -> &'static str {
    if character.flags.contains(CharacterFlags::MALE) {
        "his"
    } else if character.flags.contains(CharacterFlags::FEMALE) {
        "her"
    } else {
        "its"
    }
}

/// C `plr_send_inv` (`src/system/player.c`): the `SV_LOOKINV` paperdoll
/// fields (target's sprite/colors/12 worn-slot item sprites).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookCharacterPaperdoll {
    pub sprite: u32,
    pub colors: [u32; 3],
    pub worn_sprites: [u32; 12],
}

/// C `look_char`'s two `log_char` calls (`src/system/tool.c`): the `"#1"`
/// header line (name/title/level) and the `"#2"` body line (description
/// plus player-only stat lines).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookCharacterResult {
    pub header: String,
    pub body: String,
}

impl World {
    /// C `say(cn, format, ...)` (`src/system/talk.c:221`): area-fanned
    /// `"<name> says: \"<text>\""` at `say_dist` tiles. C's quote-rejecting
    /// `strchr(buf, '"')` check is commented out in `say()` (unlike
    /// `quiet_say`/`emote`/`murmur`/`whisper`), so this never rejects text -
    /// see `say_message`. Returns `false` only if `character_id` is unknown.
    pub fn npc_say(&mut self, character_id: CharacterId, text: &str) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let message = say_message(&character.name, text);
        self.pending_area_texts.push(WorldAreaText {
            x: character.x,
            y: character.y,
            max_distance: self.settings.say_dist.max(0) as u16,
            message: String::from_utf8_lossy(&message).into_owned(),
        });
        true
    }

    /// C `quiet_say(cn, format, ...)` (`src/system/talk.c:271`): same wire
    /// text as `say` but a shorter `quietsay_dist` range and a quote-reject
    /// guard. Returns `false` if the character is unknown or `text`
    /// contains a `"` (message dropped, matching C's early `return 0`).
    pub fn npc_quiet_say(&mut self, character_id: CharacterId, text: &str) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(message) = quiet_say_message(&character.name, text) else {
            return false;
        };
        self.pending_area_texts.push(WorldAreaText {
            x: character.x,
            y: character.y,
            max_distance: self.settings.quietsay_dist.max(0) as u16,
            message: String::from_utf8_lossy(&message).into_owned(),
        });
        true
    }

    /// C `emote(cn, format, ...)` (`src/system/talk.c:247`): `"<name>
    /// <text>."` fanned out at `emote_dist` tiles, quote-reject guard.
    /// Returns `false` if the character is unknown or `text` contains a
    /// `"`.
    pub fn npc_emote(&mut self, character_id: CharacterId, text: &str) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(message) = emote_message(&character.name, text) else {
            return false;
        };
        self.pending_area_texts.push(WorldAreaText {
            x: character.x,
            y: character.y,
            max_distance: self.settings.emote_dist.max(0) as u16,
            message: String::from_utf8_lossy(&message).into_owned(),
        });
        true
    }

    /// C `murmur(cn, format, ...)` (`src/system/talk.c:315`): `"<name>
    /// murmurs: \"<text>\""` fanned out at `whisper_dist` tiles (C's
    /// `murmur` reuses `whisper_dist`, it has no distance constant of its
    /// own), quote-reject guard. Returns `false` if the character is
    /// unknown or `text` contains a `"`.
    pub fn npc_murmur(&mut self, character_id: CharacterId, text: &str) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(message) = murmur_message(&character.name, text) else {
            return false;
        };
        self.pending_area_texts.push(WorldAreaText {
            x: character.x,
            y: character.y,
            max_distance: self.settings.whisper_dist.max(0) as u16,
            message: String::from_utf8_lossy(&message).into_owned(),
        });
        true
    }

    /// C `whisper(cn, format, ...)` (`src/system/talk.c:296`): `"<name>
    /// whispers: \"<text>\""` fanned out at `whisper_dist` tiles, quote-
    /// reject guard. Returns `false` if the character is unknown or `text`
    /// contains a `"`.
    pub fn npc_whisper(&mut self, character_id: CharacterId, text: &str) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(message) = whisper_message(&character.name, text) else {
            return false;
        };
        self.pending_area_texts.push(WorldAreaText {
            x: character.x,
            y: character.y,
            max_distance: self.settings.whisper_dist.max(0) as u16,
            message: String::from_utf8_lossy(&message).into_owned(),
        });
        true
    }

    pub fn notify_twocity_pick_from_character(&mut self, character_id: CharacterId) {
        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        let x = character.x;
        let y = character.y;
        self.notify_area(x, y, NT_NPC, NTID_TWOCITY_PICK, character_id.0 as i32, 0);
    }

    /// C `notify_area` (`src/system/notify.c:146-168`): an unconditional
    /// bounding-box broadcast (no `char_see_char`/visibility gate here - C
    /// applies that downstream in each driver's message consumer, e.g.
    /// `merchant_driver`'s `char_see_char(cn, co)` check). `NOTIFY_SIZE` is
    /// 32 tiles in C, giving a 65x65 box centered on `(x, y)`.
    pub fn notify_area(
        &mut self,
        x: u16,
        y: u16,
        message_type: i32,
        dat1: i32,
        dat2: i32,
        dat3: i32,
    ) {
        const NOTIFY_SIZE: u16 = 32;
        let min_x = x.saturating_sub(NOTIFY_SIZE);
        let max_x = x.saturating_add(NOTIFY_SIZE);
        let min_y = y.saturating_sub(NOTIFY_SIZE);
        let max_y = y.saturating_add(NOTIFY_SIZE);
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

    /// Byte-payload sibling of [`Self::queue_system_text`] - see
    /// [`WorldSystemTextBytes`].
    pub fn queue_system_text_bytes(&mut self, character_id: CharacterId, message: Vec<u8>) {
        self.pending_system_text_bytes.push(WorldSystemTextBytes {
            character_id,
            message,
        });
    }

    pub fn drain_pending_system_text_bytes(&mut self) -> Vec<WorldSystemTextBytes> {
        self.pending_system_text_bytes.drain(..).collect()
    }

    pub fn drain_pending_area_texts(&mut self) -> Vec<WorldAreaText> {
        self.pending_area_texts.drain(..).collect()
    }

    /// C `server_chat(channel, text)` (`src/system/chat/chat.c:827-834`).
    /// See `WorldChannelBroadcast` for the delivery semantics.
    pub fn queue_channel_broadcast(&mut self, channel: u8, message_bytes: Vec<u8>) {
        self.pending_channel_broadcasts.push(WorldChannelBroadcast {
            channel,
            message_bytes,
        });
    }

    pub fn drain_pending_channel_broadcasts(&mut self) -> Vec<WorldChannelBroadcast> {
        self.pending_channel_broadcasts.drain(..).collect()
    }

    /// C `cl_look_char` -> `look_char` (`src/system/player.c`,
    /// `src/system/tool.c`), text half. Gated the same way as C: the
    /// looker must exist and be `CF_PLAYER`, and `char_see_char` (bounds/
    /// LOS/light/stealth) must pass. `target_is_brave` is C's
    /// `DRD_RANDOMSHRINE_PPD` `DEATH_SHRINE` check and `target_mirror` is
    /// C's `ch[co].mirror` - both live in session-only `PlayerRuntime`
    /// state in this codebase, so the caller (which has access to
    /// `PlayerRuntime`) supplies them.
    ///
    /// REMAINING (not ported - no C-side data source exists yet in this
    /// codebase): labyrinth-solved count, first-kill Hell flavor text,
    /// army rank (`DRD_RANK_PPD`), PK info, clan info, club info. These
    /// are documented gaps, not silently dropped - see `PORTING_TODO.md`.
    pub fn look_character_text(
        &self,
        looker_id: CharacterId,
        target_id: CharacterId,
        target_is_brave: bool,
        target_mirror: u32,
    ) -> Option<LookCharacterResult> {
        let looker = self.characters.get(&looker_id)?;
        if !looker.flags.contains(CharacterFlags::PLAYER) {
            return None;
        }
        let target = self.characters.get(&target_id)?;
        if !char_see_char(looker, target, &self.map, self.date.daylight) {
            return None;
        }

        let title = look_character_title(target);
        let header = if target_is_brave {
            format!("#1{title}{} the Brave ({}):", target.name, target.level)
        } else {
            format!("#1{title}{} ({}):", target.name, target.level)
        };

        let mut body = format!("#2{} ", target.description);
        if target.flags.contains(CharacterFlags::PLAYER) {
            if target.flags.contains(CharacterFlags::HARDCORE) {
                body.push_str(&format!(
                    "{} is a hardcore character and died {} times. ",
                    look_character_hename(target),
                    target.deaths
                ));
            } else {
                let plural = if target.saves == 1 { "" } else { "s" };
                body.push_str(&format!(
                    "{} has {} save{plural}, was saved {} times and died {} times. ",
                    look_character_hename(target),
                    target.saves,
                    target.got_saved,
                    target.deaths
                ));
            }
        }

        for (index, &value) in target.professions.iter().enumerate() {
            if value == 0 {
                continue;
            }
            if let Some(&(name, max)) = PROF_TABLE.get(index) {
                body.push_str(&format!(
                    "{} is {}{name}. ",
                    look_character_hename(target),
                    profession_title(value, max)
                ));
            }
        }

        if target.flags.contains(CharacterFlags::PLAYER) {
            body.push_str(&format!("Mirror={target_mirror}. "));
            body.push_str(&format!("Karma: {}", target.karma));
        }

        Some(LookCharacterResult { header, body })
    }

    /// C `plr_send_inv` (`src/system/player.c`): builds the `SV_LOOKINV`
    /// paperdoll fields directly from the target's `sprite`/`c1`/`c2`/`c3`
    /// and the 12 worn-equipment slot item sprites (0 for empty slots).
    /// No visibility gate here - callers must already have confirmed
    /// `char_see_char` via `look_character_text`.
    pub fn look_character_paperdoll(
        &self,
        target_id: CharacterId,
    ) -> Option<LookCharacterPaperdoll> {
        let target = self.characters.get(&target_id)?;
        let mut worn_sprites = [0u32; 12];
        for (slot, sprite) in worn_sprites.iter_mut().enumerate() {
            *sprite = target
                .inventory
                .get(slot)
                .copied()
                .flatten()
                .and_then(|item_id| self.items.get(&item_id))
                .map(|item| item.sprite.max(0) as u32)
                .unwrap_or(0);
        }
        Some(LookCharacterPaperdoll {
            sprite: target.sprite.max(0) as u32,
            colors: [
                u32::from(target.c1),
                u32::from(target.c2),
                u32::from(target.c3),
            ],
            worn_sprites,
        })
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
