use serde::{Deserialize, Serialize};

use crate::player::{make_drd, DEV_ID_DB};

pub const MAX_TELLS: usize = 10;
pub const DRD_TELL_DATA: u32 = make_drd(DEV_ID_DB, 101);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TellSlot {
    pub target_id: u32,
    pub tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TellData {
    pub slots: [TellSlot; MAX_TELLS],
}

impl Default for TellData {
    fn default() -> Self {
        Self {
            slots: [TellSlot::default(); MAX_TELLS],
        }
    }
}

impl TellData {
    pub fn register_sent_tell(&mut self, target_id: u32, current_tick: u64) {
        let mut empty = None;

        for (index, slot) in self.slots.iter().enumerate() {
            if slot.target_id == target_id {
                return;
            }
            if slot.target_id == 0 && empty.is_none() {
                empty = Some(index);
            }
        }

        if let Some(index) = empty {
            self.slots[index] = TellSlot {
                target_id,
                tick: current_tick,
            };
        }
    }

    pub fn register_received_tell(&mut self, target_id: u32) {
        for slot in &mut self.slots {
            if slot.target_id == target_id {
                *slot = TellSlot::default();
            }
        }
    }

    pub fn check_tells(&mut self, current_tick: u64, ticks_per_second: u64) -> Vec<u32> {
        let mut expired = Vec::new();

        for slot in &mut self.slots {
            if slot.target_id != 0 && current_tick.saturating_sub(slot.tick) > ticks_per_second {
                expired.push(slot.target_id);
                *slot = TellSlot::default();
            }
        }

        expired
    }
}

pub fn tell_not_listening_message(name: &str) -> Vec<u8> {
    crate::log_text::sanitize_log_bytes(format!("{name} is not listening.").as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tell_data_drd_matches_legacy_header() {
        assert_eq!(DRD_TELL_DATA, 0x0100_0065);
    }

    #[test]
    fn sent_tells_keep_first_empty_slot_and_ignore_duplicates() {
        let mut tells = TellData::default();

        tells.register_sent_tell(10, 100);
        tells.register_sent_tell(11, 101);
        tells.register_sent_tell(10, 200);

        assert_eq!(
            tells.slots[0],
            TellSlot {
                target_id: 10,
                tick: 100
            }
        );
        assert_eq!(
            tells.slots[1],
            TellSlot {
                target_id: 11,
                tick: 101
            }
        );
        assert_eq!(tells.slots[2], TellSlot::default());
    }

    #[test]
    fn sent_tells_ignore_new_targets_when_full() {
        let mut tells = TellData::default();
        for target_id in 1..=MAX_TELLS as u32 {
            tells.register_sent_tell(target_id, target_id as u64);
        }

        tells.register_sent_tell(99, 999);

        assert!(!tells.slots.iter().any(|slot| slot.target_id == 99));
    }

    #[test]
    fn received_tell_removes_all_matching_targets() {
        let mut tells = TellData::default();
        tells.slots[0] = TellSlot {
            target_id: 7,
            tick: 1,
        };
        tells.slots[1] = TellSlot {
            target_id: 8,
            tick: 2,
        };
        tells.slots[2] = TellSlot {
            target_id: 7,
            tick: 3,
        };

        tells.register_received_tell(7);

        assert_eq!(tells.slots[0], TellSlot::default());
        assert_eq!(
            tells.slots[1],
            TellSlot {
                target_id: 8,
                tick: 2
            }
        );
        assert_eq!(tells.slots[2], TellSlot::default());
    }

    #[test]
    fn check_tells_uses_strict_one_second_timeout_and_clears_expired_slots() {
        let mut tells = TellData::default();
        tells.register_sent_tell(10, 100);
        tells.register_sent_tell(11, 101);

        assert!(tells.check_tells(110, 10).is_empty());
        assert_eq!(tells.check_tells(112, 10), vec![10, 11]);
        assert_eq!(tells.slots[0], TellSlot::default());
        assert_eq!(tells.slots[1], TellSlot::default());
    }

    #[test]
    fn not_listening_message_matches_legacy_text() {
        assert_eq!(
            tell_not_listening_message("Bob"),
            b"Bob is not listening.".to_vec()
        );
    }
}
