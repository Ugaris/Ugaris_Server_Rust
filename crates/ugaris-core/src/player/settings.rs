use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandAlias {
    pub from: String,
    pub to: String,
}

impl PlayerRuntime {
    pub fn encode_legacy_alias_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_ALIAS_PPD_SIZE];
        for (index, alias) in self.aliases.iter().take(ALIAS_MAX_ENTRIES).enumerate() {
            let offset = index * (ALIAS_FROM_LEN + ALIAS_TO_LEN);
            write_c_string(&mut bytes, offset, ALIAS_FROM_LEN, &alias.from);
            write_c_string(&mut bytes, offset + ALIAS_FROM_LEN, ALIAS_TO_LEN, &alias.to);
        }
        bytes
    }

    pub fn decode_legacy_alias_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_ALIAS_PPD_SIZE {
            return false;
        }
        self.aliases.clear();
        for index in 0..ALIAS_MAX_ENTRIES {
            let offset = index * (ALIAS_FROM_LEN + ALIAS_TO_LEN);
            let from = read_c_string(bytes, offset, ALIAS_FROM_LEN);
            if from.is_empty() {
                continue;
            }
            let to = read_c_string(bytes, offset + ALIAS_FROM_LEN, ALIAS_TO_LEN);
            self.aliases.push(CommandAlias { from, to });
        }
        true
    }

    pub fn expand_aliases(&self, source: &str) -> String {
        fn alias_stop(ch: char) -> bool {
            ch.is_whitespace() || (ch.is_ascii_punctuation() && ch != '\'')
        }

        let mut out = String::new();
        let mut token = String::new();
        for ch in source.chars() {
            if alias_stop(ch) {
                if token.is_empty() {
                    out.push(ch);
                    continue;
                }
                if let Some(alias) = self
                    .aliases
                    .iter()
                    .find(|alias| alias.from.eq_ignore_ascii_case(&token))
                {
                    out.push_str(&alias.to);
                } else {
                    out.push_str(&token);
                }
                token.clear();
                out.push(ch);
            } else {
                token.push(ch);
            }
            if out.len() > 198 {
                out.truncate(199);
                return out;
            }
        }
        if !token.is_empty() {
            if let Some(alias) = self
                .aliases
                .iter()
                .find(|alias| alias.from.eq_ignore_ascii_case(&token))
            {
                out.push_str(&alias.to);
            } else {
                out.push_str(&token);
            }
        }
        if out.len() > 199 {
            out.truncate(199);
        }
        out
    }

    pub fn ignores_character(&self, character_id: u32) -> bool {
        character_id != 0 && self.ignored_characters.contains(&character_id)
    }

    pub fn toggle_ignored_character(&mut self, character_id: u32) -> IgnoreToggleResult {
        if character_id == 0 {
            return IgnoreToggleResult::Full;
        }
        if let Some(index) = self
            .ignored_characters
            .iter()
            .position(|ignored| *ignored == character_id)
        {
            self.ignored_characters.remove(index);
            return IgnoreToggleResult::Removed;
        }
        if self.ignored_characters.len() >= IGNORE_MAX_ENTRIES {
            return IgnoreToggleResult::Full;
        }
        self.ignored_characters.push(character_id);
        IgnoreToggleResult::Added
    }

    pub fn clear_ignored_characters(&mut self) {
        self.ignored_characters.clear();
    }

    pub fn encode_legacy_ignore_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_IGNORE_PPD_SIZE];
        for (index, character_id) in self
            .ignored_characters
            .iter()
            .copied()
            .take(IGNORE_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                index * 4,
                character_id.min(i32::MAX as u32) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_ignore_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_IGNORE_PPD_SIZE {
            return false;
        }
        self.ignored_characters.clear();
        for index in 0..IGNORE_MAX_ENTRIES {
            let character_id = read_i32(bytes, index * 4);
            if character_id > 0 {
                self.ignored_characters.push(character_id as u32);
            }
        }
        true
    }

    pub fn set_max_lag_seconds(&mut self, seconds: u8) {
        self.max_lag_seconds = seconds;
    }

    pub fn toggle_hints(&mut self) -> bool {
        self.hints_disabled = !self.hints_disabled;
        self.hints_disabled
    }

    pub fn toggle_autoturn(&mut self) -> bool {
        self.autoturn_enabled = !self.autoturn_enabled;
        self.autoturn_enabled
    }

    /// C `fight_driver_attack_visible`'s own `ppd->nobless`/.../`ppd->
    /// nopulse` positional argument list (`src/system/drvlib.c:2260-2263`,
    /// only reachable for `ch[cn].flags & CF_PLAYER`) plus the `nomove`
    /// argument threaded in separately by both of its callers
    /// (`lostcon_driver`'s own `ppd->nomove` and the not-yet-wired normal
    /// player tick). `FightDriverSuppressions::nofreeze`/`nopulse` map to
    /// C's `ppd->nofreeze`/`ppd->nopulse` the same way.
    pub fn fight_driver_suppressions(&self) -> crate::world::FightDriverSuppressions {
        crate::world::FightDriverSuppressions {
            nomove: self.no_move,
            nobless: self.no_bless,
            noheal: self.no_heal,
            noflash: self.no_flash,
            nofireball: self.no_fireball,
            noball: self.no_ball,
            noshield: self.no_shield,
            nowarcry: self.no_warcry,
            nofreeze: self.no_freeze,
            nopulse: self.no_pulse,
        }
    }

    /// C `lostcon_driver`'s own six self-care toggles
    /// (`src/module/lostcon.c:164-218`), as opposed to the ten
    /// `fight_driver_attack_enemy` toggles `fight_driver_suppressions`
    /// maps. `nomove`/`noflash`/`nofireball`/`noball`/`nowarcry`/
    /// `nofreeze`/`nopulse`/`norecall` have no `LostconSelfCareSuppressions`
    /// field - they are consumed only by the fight-driver engine or by
    /// still-unported callers (`/norecall`'s `player_use_recall`).
    pub fn lostcon_self_care_suppressions(&self) -> crate::world::LostconSelfCareSuppressions {
        crate::world::LostconSelfCareSuppressions {
            noheal: self.no_heal,
            noshield: self.no_shield,
            nobless: self.no_bless,
            nolife: self.no_life,
            nomana: self.no_mana,
            nocombo: self.no_combo,
        }
    }

    /// True if any of the 16 lag-control/automation toggles (everything
    /// but `autoturn`/`maxlag`/`hints`, which each have their own
    /// pre-existing "is this default" gate) is non-default, matching the
    /// `!had_lostcon && ...` fresh-block-write condition below.
    pub(crate) fn has_nondefault_lag_control_toggle(&self) -> bool {
        self.autobless_enabled
            || self.autopulse_enabled
            || self.no_ball
            || self.no_bless
            || self.no_fireball
            || self.no_flash
            || self.no_freeze
            || self.no_heal
            || self.no_shield
            || self.no_warcry
            || self.no_life
            || self.no_mana
            || self.no_combo
            || self.no_move
            || self.no_pulse
            || self.no_recall
    }

    pub fn encode_legacy_lostcon_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_LOSTCON_PPD_SIZE];
        write_i32(
            &mut bytes,
            LOSTCON_PPD_AUTOBLESS_OFFSET,
            i32::from(self.autobless_enabled),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_AUTOPULSE_OFFSET,
            i32::from(self.autopulse_enabled),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_NOBLESS_OFFSET,
            i32::from(self.no_bless),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_NOHEAL_OFFSET,
            i32::from(self.no_heal),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_NOFLASH_OFFSET,
            i32::from(self.no_flash),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_NOFIREBALL_OFFSET,
            i32::from(self.no_fireball),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_NOBALL_OFFSET,
            i32::from(self.no_ball),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_NOSHIELD_OFFSET,
            i32::from(self.no_shield),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_NOWARCRY_OFFSET,
            i32::from(self.no_warcry),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_NOFREEZE_OFFSET,
            i32::from(self.no_freeze),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_NOMANA_OFFSET,
            i32::from(self.no_mana),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_NOLIFE_OFFSET,
            i32::from(self.no_life),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_NOCOMBO_OFFSET,
            i32::from(self.no_combo),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_NOMOVE_OFFSET,
            i32::from(self.no_move),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_NOPULSE_OFFSET,
            i32::from(self.no_pulse),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_NORECALL_OFFSET,
            i32::from(self.no_recall),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_AUTOTURN_OFFSET,
            i32::from(self.autoturn_enabled),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_MAXLAG_OFFSET,
            i32::from(self.max_lag_seconds),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_HINTS_OFFSET,
            i32::from(self.hints_disabled),
        );
        bytes
    }

    pub fn decode_legacy_lostcon_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_LOSTCON_PPD_SIZE {
            return false;
        }
        self.autobless_enabled = read_i32(bytes, LOSTCON_PPD_AUTOBLESS_OFFSET) != 0;
        self.autopulse_enabled = read_i32(bytes, LOSTCON_PPD_AUTOPULSE_OFFSET) != 0;
        self.no_bless = read_i32(bytes, LOSTCON_PPD_NOBLESS_OFFSET) != 0;
        self.no_heal = read_i32(bytes, LOSTCON_PPD_NOHEAL_OFFSET) != 0;
        self.no_flash = read_i32(bytes, LOSTCON_PPD_NOFLASH_OFFSET) != 0;
        self.no_fireball = read_i32(bytes, LOSTCON_PPD_NOFIREBALL_OFFSET) != 0;
        self.no_ball = read_i32(bytes, LOSTCON_PPD_NOBALL_OFFSET) != 0;
        self.no_shield = read_i32(bytes, LOSTCON_PPD_NOSHIELD_OFFSET) != 0;
        self.no_warcry = read_i32(bytes, LOSTCON_PPD_NOWARCRY_OFFSET) != 0;
        self.no_freeze = read_i32(bytes, LOSTCON_PPD_NOFREEZE_OFFSET) != 0;
        self.no_mana = read_i32(bytes, LOSTCON_PPD_NOMANA_OFFSET) != 0;
        self.no_life = read_i32(bytes, LOSTCON_PPD_NOLIFE_OFFSET) != 0;
        self.no_combo = read_i32(bytes, LOSTCON_PPD_NOCOMBO_OFFSET) != 0;
        self.no_move = read_i32(bytes, LOSTCON_PPD_NOMOVE_OFFSET) != 0;
        self.no_pulse = read_i32(bytes, LOSTCON_PPD_NOPULSE_OFFSET) != 0;
        self.no_recall = read_i32(bytes, LOSTCON_PPD_NORECALL_OFFSET) != 0;
        self.max_lag_seconds =
            read_i32(bytes, LOSTCON_PPD_MAXLAG_OFFSET).clamp(0, i32::from(u8::MAX)) as u8;
        self.hints_disabled = read_i32(bytes, LOSTCON_PPD_HINTS_OFFSET) != 0;
        self.autoturn_enabled = read_i32(bytes, LOSTCON_PPD_AUTOTURN_OFFSET) != 0;
        true
    }

    pub fn encode_legacy_swear_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_SWEAR_PPD_SIZE];
        let copy_len = self.swear_ppd.len().min(LEGACY_SWEAR_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.swear_ppd[..copy_len]);
        write_i32(
            &mut bytes,
            SWEAR_PPD_BANNED_TILL_OFFSET,
            self.shutup_until_seconds.min(i32::MAX as u64) as i32,
        );
        bytes
    }

    pub fn decode_legacy_swear_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_SWEAR_PPD_SIZE {
            return false;
        }
        self.swear_ppd = bytes[..LEGACY_SWEAR_PPD_SIZE].to_vec();
        self.shutup_until_seconds = read_i32(bytes, SWEAR_PPD_BANNED_TILL_OFFSET).max(0) as u64;
        true
    }

    /// C `char_swap`'s `ppd->swapped = realtime;` (`do.c:1671-1673`, only
    /// reached when the swap-initiating character is `CF_PLAYER`), stamping
    /// the real-time-seconds timestamp read by the give-item anti-scam
    /// cooldown check (`do.c:511-514`: `realtime - ppd->swapped < 20`
    /// blocks giving an item to a character who swapped places in the last
    /// 20 seconds; that read side isn't ported yet).
    pub fn record_swap(&mut self, realtime_seconds: i32) {
        if self.misc_ppd.len() < LEGACY_MISC_PPD_SIZE {
            self.misc_ppd.resize(LEGACY_MISC_PPD_SIZE, 0);
        }
        write_i32(
            &mut self.misc_ppd,
            MISC_PPD_SWAPPED_OFFSET,
            realtime_seconds,
        );
    }

    /// Reads back the `char_swap` timestamp set by [`Self::record_swap`].
    /// Returns `0` (matching a freshly zeroed C `struct misc_ppd`) if no
    /// swap has ever been recorded.
    pub fn swapped_at(&self) -> i32 {
        if self.misc_ppd.len() < LEGACY_MISC_PPD_SIZE {
            return 0;
        }
        read_i32(&self.misc_ppd, MISC_PPD_SWAPPED_OFFSET)
    }

    /// C `cmd_complain`'s `ppd->complaint_date` read (`system/command.c:
    /// 2287,2306`). `0` (matching a freshly zeroed C `struct misc_ppd`)
    /// means the caller has never seen the `/complain` disclaimer yet.
    pub fn complaint_date(&self) -> i32 {
        if self.misc_ppd.len() < LEGACY_MISC_PPD_SIZE {
            return 0;
        }
        read_i32(&self.misc_ppd, MISC_PPD_COMPLAINT_DATE_OFFSET)
    }

    /// C `cmd_complain`'s three `ppd->complaint_date = ...` writes
    /// (`system/command.c:2306,2308,2347`): `1` after the caller sees the
    /// one-time disclaimer, `realtime` both when a rate-limited retry is
    /// rejected (a real C quirk - this resets the cooldown window on every
    /// rejected attempt, not just on a successful complaint) and when a
    /// complaint is actually sent.
    pub fn record_complaint(&mut self, value: i32) {
        if self.misc_ppd.len() < LEGACY_MISC_PPD_SIZE {
            self.misc_ppd.resize(LEGACY_MISC_PPD_SIZE, 0);
        }
        write_i32(&mut self.misc_ppd, MISC_PPD_COMPLAINT_DATE_OFFSET, value);
    }
}
