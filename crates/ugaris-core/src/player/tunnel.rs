use super::*;

impl PlayerRuntime {
    pub fn encode_legacy_tunnel_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_TUNNEL_PPD_SIZE];
        let copy_len = self.tunnel_ppd.len().min(LEGACY_TUNNEL_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.tunnel_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_tunnel_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_TUNNEL_PPD_SIZE {
            return false;
        }
        self.tunnel_ppd = bytes[..LEGACY_TUNNEL_PPD_SIZE].to_vec();
        true
    }

    /// C `tunnel_ppd::clevel` (`tunnel.h:7`): the player's currently
    /// assigned tunnel dungeon level (`0` for a freshly zeroed struct,
    /// matching an un-set `set_data`).
    pub fn tunnel_clevel(&self) -> i32 {
        if self.tunnel_ppd.len() < LEGACY_TUNNEL_PPD_SIZE {
            return 0;
        }
        read_i32(&self.tunnel_ppd, 0)
    }

    /// Writes `tunnel_ppd::clevel`, growing the backing store to
    /// [`LEGACY_TUNNEL_PPD_SIZE`] on first use (matching C's zero-
    /// initializing `set_data`). C `/changetunnel` (`command.c:2045-2085`).
    pub fn set_tunnel_clevel(&mut self, value: i32) {
        if self.tunnel_ppd.len() < LEGACY_TUNNEL_PPD_SIZE {
            self.tunnel_ppd.resize(LEGACY_TUNNEL_PPD_SIZE, 0);
        }
        write_i32(&mut self.tunnel_ppd, 0, value);
    }

    /// C `tunnel_ppd::used[level]` (`tunnel.h:8`): the number of rewarded
    /// completions recorded at `level` (`0` for a level never touched, or
    /// for an out-of-range/negative `level`, matching a freshly zeroed C
    /// struct - `set_data` would have zero-initialized it too).
    pub fn tunnel_used(&self, level: i32) -> u8 {
        if level < 0 {
            return 0;
        }
        let idx = TUNNEL_PPD_USED_BASE_OFFSET + level as usize;
        if idx >= self.tunnel_ppd.len() {
            return 0;
        }
        self.tunnel_ppd[idx]
    }

    /// Writes `tunnel_ppd::used[level]`, growing the backing store to
    /// [`LEGACY_TUNNEL_PPD_SIZE`] on first use (matching C's zero-
    /// initializing `set_data`). No-op for a negative `level`.
    pub fn set_tunnel_used(&mut self, level: i32, value: u8) {
        if level < 0 {
            return;
        }
        let idx = TUNNEL_PPD_USED_BASE_OFFSET + level as usize;
        if self.tunnel_ppd.len() <= idx {
            self.tunnel_ppd
                .resize(LEGACY_TUNNEL_PPD_SIZE.max(idx + 1), 0);
        }
        self.tunnel_ppd[idx] = value;
    }

    pub fn encode_legacy_gorwin_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_GORWIN_PPD_SIZE];
        let copy_len = self.gorwin_ppd.len().min(LEGACY_GORWIN_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.gorwin_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_gorwin_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_GORWIN_PPD_SIZE {
            return false;
        }
        self.gorwin_ppd = bytes[..LEGACY_GORWIN_PPD_SIZE].to_vec();
        true
    }

    /// C `gorwin_ppd::tunnel_level` (`tunnel.h:12`): `0` means "not yet
    /// initialized" (`initialize_gorwin_ppd`, not yet ported), matching a
    /// freshly zeroed C struct.
    pub fn gorwin_tunnel_level(&self) -> i32 {
        if self.gorwin_ppd.len() < LEGACY_GORWIN_PPD_SIZE {
            return 0;
        }
        read_i32(&self.gorwin_ppd, 0)
    }

    /// Writes `gorwin_ppd::tunnel_level`, growing the backing store to
    /// [`LEGACY_GORWIN_PPD_SIZE`] on first use (matching C's zero-
    /// initializing `set_data`).
    pub fn set_gorwin_tunnel_level(&mut self, value: i32) {
        if self.gorwin_ppd.len() < LEGACY_GORWIN_PPD_SIZE {
            self.gorwin_ppd.resize(LEGACY_GORWIN_PPD_SIZE, 0);
        }
        write_i32(&mut self.gorwin_ppd, 0, value);
    }
}

/// C `find_next_available_level` (`src/area/33/tunnel.c:516-525`): pure
/// scan for the next tunnel level above `start_level` (up to
/// `min(MAX_TUNNEL_LEVEL, max_level)`) with `used[level] < MAX_TUNNEL_USES`
/// completions recorded. `tunnel_used` is a `PlayerRuntime::tunnel_used`
/// snapshot indexed by level directly (`tunnel_ppd::used[204]`), the same
/// shape both call sites already needed to pre-fetch since neither has a
/// live `&PlayerRuntime` at hand: `world::npc::area33::gorwin`'s
/// character-driver tick (`GorwinPlayerFacts::tunnel_used`) and
/// `world::tunnel`'s `IDR_TUNNELDOOR` `give_reward` port
/// (`TunnelRewardFacts::tunnel_used`).
pub fn find_next_available_tunnel_level(
    tunnel_used: &[u8],
    start_level: i32,
    max_level: i32,
) -> Option<i32> {
    let upper = MAX_TUNNEL_LEVEL.min(max_level);
    ((start_level + 1)..=upper)
        .find(|&level| tunnel_used.get(level as usize).copied().unwrap_or(0) < MAX_TUNNEL_USES)
}
