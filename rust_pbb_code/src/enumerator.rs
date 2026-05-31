use rand::Rng;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct CharsetEntry {
    pub bytes: Vec<u8>,
    pub len: usize,
}

#[derive(Clone, Debug)]
pub enum EnumMode {
    Sequential,
    RandomInterval,
    BitwiseRandom,
    RandomIntervalPaired,
}

#[derive(Clone, Debug)]
pub struct PrefixSuffix {
    pub bytes: Vec<u8>,
    pub len: usize,
}

#[derive(Clone)]
pub struct NameConfig {
    pub team_bytes: Vec<u8>,
    pub team_len: usize,
    pub prefixes: Vec<PrefixSuffix>,
    pub suffixes: Vec<PrefixSuffix>,
    pub charset: Vec<CharsetEntry>,
    pub var_len: usize,
    pub enum_mode: EnumMode,
    pub charset_total: usize,
}

impl NameConfig {
    /// Total combinations = charset_size^var_len
    pub fn total_combinations(&self) -> usize {
        self.charset_total.pow(self.var_len as u32)
    }
}

/// Generate names in sequential mode
/// Returns an iterator that yields name strings
pub struct SequentialGenerator {
    config: NameConfig,
    current: Vec<usize>,
    prefix_idx: usize,
    suffix_idx: usize,
    done: bool,
    total: u64,
    count: u64,
}

impl SequentialGenerator {
    pub fn new(config: NameConfig, total: u64) -> Self {
        let current = vec![0usize; config.var_len];
        Self {
            config,
            current,
            prefix_idx: 0,
            suffix_idx: 0,
            done: false,
            total,
            count: 0,
        }
    }

    /// Generate the next batch of names (up to batch_size)
    pub fn next_batch(&mut self, batch_size: usize) -> Vec<String> {
        let mut result = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            if self.done || self.count >= self.total {
                break;
            }
            if let Some(name) = self.next_name() {
                result.push(name);
                self.count += 1;
            }
        }
        result
    }

    fn next_name(&mut self) -> Option<String> {
        if self.current.is_empty() {
            return None;
        }

        let prefix = &self.config.prefixes[self.prefix_idx % self.config.prefixes.len()];
        let suffix = &self.config.suffixes[self.suffix_idx % self.config.suffixes.len()];

        // Build name bytes
        let mut name_bytes: Vec<u8> = Vec::new();
        name_bytes.extend_from_slice(&prefix.bytes);
        for idx in &self.current {
            let entry = &self.config.charset[*idx];
            name_bytes.extend_from_slice(&entry.bytes);
        }
        name_bytes.extend_from_slice(&suffix.bytes);

        let name_str = String::from_utf8_lossy(&name_bytes).to_string();

        // Increment counter
        self.increment();

        Some(name_str)
    }

    fn increment(&mut self) {
        for i in (0..self.current.len()).rev() {
            self.current[i] += 1;
            if self.current[i] < self.config.charset.len() {
                return;
            }
            self.current[i] = 0;
        }
        // Overflow: move to next prefix/suffix combination
        self.suffix_idx += 1;
        if self.suffix_idx >= self.config.suffixes.len() {
            self.suffix_idx = 0;
            self.prefix_idx += 1;
            if self.prefix_idx >= self.config.prefixes.len() {
                self.done = true;
            }
        }
    }
}

/// Generate names in random mode (random intervals of 1e6)
pub struct RandomIntervalGenerator {
    config: NameConfig,
    intervals: Vec<u64>,
    current_interval: usize,
    current_pos: u64,
    count: u64,
    total: u64,
}

impl RandomIntervalGenerator {
    pub fn new(config: NameConfig, total: u64) -> Self {
        let mut rng = rand::thread_rng();
        let interval_size = 1_000_000u64;
        let num_intervals = ((total + interval_size - 1) / interval_size) as usize;

        let mut intervals: Vec<u64> = (0..num_intervals as u64).collect();
        // Fisher-Yates shuffle
        for i in (1..intervals.len()).rev() {
            let j = rng.gen_range(0..=i);
            intervals.swap(i, j);
        }

        Self {
            config,
            intervals,
            current_interval: 0,
            current_pos: 0,
            count: 0,
            total,
        }
    }

    pub fn next_batch(&mut self, batch_size: usize) -> Vec<String> {
        let mut result = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            if self.count >= self.total {
                break;
            }
            if let Some(name) = self.next_name() {
                result.push(name);
                self.count += 1;
            }
        }
        result
    }

    fn next_name(&mut self) -> Option<String> {
        if self.current_interval >= self.intervals.len() {
            return None;
        }

        let interval_size = 1_000_000u64;
        let base = self.intervals[self.current_interval] * interval_size + self.current_pos;

        // Convert base to charset index representation
        let mut idx = base;
        let mut char_indices = vec![0usize; self.config.var_len];
        let charset_len = self.config.charset.len() as u64;

        for i in (0..self.config.var_len).rev() {
            char_indices[i] = (idx % charset_len) as usize;
            idx /= charset_len;
        }

        let prefix = &self.config.prefixes[0];
        let suffix = &self.config.suffixes[0];

        let mut name_bytes: Vec<u8> = Vec::new();
        name_bytes.extend_from_slice(&prefix.bytes);
        for ci in &char_indices {
            let entry = &self.config.charset[*ci];
            name_bytes.extend_from_slice(&entry.bytes);
        }
        name_bytes.extend_from_slice(&suffix.bytes);

        self.current_pos += 1;
        if self.current_pos >= interval_size {
            self.current_interval += 1;
            self.current_pos = 0;
        }

        Some(String::from_utf8_lossy(&name_bytes).to_string())
    }
}

/// Fast property-only computation for miners
/// Computes namebase and properties without full skill computation
pub fn compute_namebase_and_props_fast(
    team_bytes: &[u8],
    name_body_bytes: &[u8],
) -> Option<[u8; 128]> {
    if team_bytes.is_empty() || name_body_bytes.is_empty() {
        return None;
    }

    let mut val: [u8; 256] = std::array::from_fn(|i| i as u8);

    // Shuffle with team
    shuffle_val(&mut val, team_bytes);
    // Shuffle with name body twice
    shuffle_val(&mut val, name_body_bytes);
    shuffle_val(&mut val, name_body_bytes);

    // Compute namebase
    let mut namebase = [0u8; 128];
    let mut bonus_len = 0;
    for i in 0..256 {
        let m = val[i].wrapping_mul(181).wrapping_add(160);
        if m >= 89 && m < 217 {
            namebase[bonus_len] = m & 63;
            bonus_len += 1;
        }
    }

    Some(namebase)
}

fn shuffle_val(val: &mut [u8; 256], bytes: &[u8]) {
    let mut s: u8 = 0;
    for i in 0..256 {
        s = s.wrapping_add(bytes[i % bytes.len()].wrapping_add(val[i]));
        val.swap(i, s as usize);
    }
}

/// Fast ba_wei from namebase (first 32 items only)
pub fn compute_ba_wei_fast(namebase: &[u8; 128]) -> i32 {
    let mut r: [u8; 32] = [0; 32];
    r.copy_from_slice(&namebase[..32]);

    // HP
    r[0..10].sort();
    let hp = 154 + r[3] as i32 + r[4] as i32 + r[5] as i32 + r[6] as i32;

    // Other 7 stats
    let mut seven_sum = 0i32;
    let stat_indices: [(usize, usize, usize); 7] = [
        (10, 11, 12), (13, 14, 15), (16, 17, 18), (19, 20, 21),
        (22, 23, 24), (25, 26, 27), (28, 29, 30),
    ];

    for &(a, b, c) in &stat_indices {
        let arr = [r[a], r[b], r[c]];
        seven_sum += median_u8(arr) as i32 + 36;
    }

    seven_sum + hp / 3
}

fn median_u8(arr: [u8; 3]) -> u8 {
    let mut a = arr;
    a.sort();
    a[1]
}

/// Full name computation (with skills) for output
pub fn compute_full_name(team_bytes: &[u8], name_body_bytes: &[u8]) -> Option<String> {
    use crate::namecalc::load_name;

    let name_str = String::from_utf8_lossy(name_body_bytes);
    let full = if team_bytes == name_body_bytes {
        name_str.to_string()
    } else {
        format!("{}@{}", name_str, String::from_utf8_lossy(team_bytes))
    };

    if load_name(&full).is_some() {
        Some(full)
    } else {
        None
    }
}

/// Preset character sets
pub fn preset_charsets() -> Vec<(String, Vec<CharsetEntry>)> {
    let mut presets = Vec::new();

    // Lowercase Greek (24 chars, each 2 bytes in UTF-8)
    let greek_lower: Vec<CharsetEntry> = (0x03B1..=0x03C9)
        .filter(|&c| c != 0x03C2) // skip final sigma
        .map(|c| {
            let s = char::from_u32(c).unwrap().to_string();
            CharsetEntry {
                len: s.len(),
                bytes: s.into_bytes(),
            }
        })
        .collect();
    presets.push(("小写希腊字母".to_string(), greek_lower));

    // Uppercase Greek (24 chars)
    let greek_upper: Vec<CharsetEntry> = (0x0391..=0x03A9)
        .filter(|&c| c != 0x03A2) // skip unused
        .map(|c| {
            let s = char::from_u32(c).unwrap().to_string();
            CharsetEntry {
                len: s.len(),
                bytes: s.into_bytes(),
            }
        })
        .collect();
    presets.push(("大写希腊字母".to_string(), greek_upper));

    // Lowercase Latin (26 chars)
    let latin_lower: Vec<CharsetEntry> = (b'a'..=b'z')
        .map(|c| CharsetEntry {
            len: 1,
            bytes: vec![c],
        })
        .collect();
    presets.push(("小写拉丁字母".to_string(), latin_lower));

    // Uppercase Latin (26 chars)
    let latin_upper: Vec<CharsetEntry> = (b'A'..=b'Z')
        .map(|c| CharsetEntry {
            len: 1,
            bytes: vec![c],
        })
        .collect();
    presets.push(("大写拉丁字母".to_string(), latin_upper));

    // Digits (10 chars)
    let digits: Vec<CharsetEntry> = (b'0'..=b'9')
        .map(|c| CharsetEntry {
            len: 1,
            bytes: vec![c],
        })
        .collect();
    presets.push(("数字".to_string(), digits));

    // All printable ASCII (95 chars)
    let ascii: Vec<CharsetEntry> = (33u8..=126)
        .filter(|&c| c != b'@' && c != b'+' && c != b'\\' && c != b'"')
        .map(|c| CharsetEntry {
            len: 1,
            bytes: vec![c],
        })
        .collect();
    presets.push(("可打印ASCII".to_string(), ascii));

    presets
}
