/// Rand structure - the PRNG used for skill shuffling
/// Matches the C++ struct: a, b (indices), c (256-byte permutation table)
#[derive(Clone)]
pub struct Rand {
    a: usize,
    b: usize,
    c: [u8; 256],
}

impl Rand {
    pub fn new(_a: u8, _b: u8, val: &[u8; 256]) -> Self {
        let a = _a as usize;
        let b = _b as usize;
        let mut c = [0u8; 256];
        c.copy_from_slice(val);
        Self { a, b, c }
    }

    pub fn next_byte(&mut self) -> u8 {
        self.a = (self.a + 1) % 256;
        self.b = (self.b + self.c[self.a] as usize) % 256;
        self.c.swap(self.a, self.b);
        self.c[(self.c[self.a] as usize + self.c[self.b] as usize) & 0xFF]
    }

    pub fn next(&mut self, limit: i32) -> i32 {
        if limit == 0 {
            return 0;
        }
        let mut u = self.next_byte() as i32;
        u = ((u << 8) | self.next_byte() as i32) % limit;
        u
    }
}

/// Skill structure
#[derive(Clone, Debug, Default)]
pub struct Skill {
    pub id: i32,
    pub freq: i32,
    pub gained: bool,
}

/// Name structure - represents a full name with team
#[derive(Clone)]
pub struct Name {
    pub val: [u8; 256],
    pub namebase: [u8; 128],
    pub namebonus: [u8; 128],
    pub props: [i32; 8], // HP, Atk, Def, Spd, Dex, Mag, Res, Int
    pub skills: [Skill; 16],
    pub skill_order: [i32; 16], // skill IDs for each of the 16 slots
    pub raw_name: String,
}

impl Name {
    pub fn new() -> Self {
        Self {
            val: [0u8; 256],
            namebase: [0u8; 128],
            namebonus: [0u8; 128],
            props: [0i32; 8],
            skills: [Skill::default(); 16],
            skill_order: [0i32; 16],
            raw_name: String::new(),
        }
    }

    pub fn hp(&self) -> i32 { self.props[0] }
    pub fn atk(&self) -> i32 { self.props[1] }
    pub fn def(&self) -> i32 { self.props[2] }
    pub fn spd(&self) -> i32 { self.props[3] }
    pub fn dex(&self) -> i32 { self.props[4] }
    pub fn mag(&self) -> i32 { self.props[5] }
    pub fn res(&self) -> i32 { self.props[6] }
    pub fn int(&self) -> i32 { self.props[7] }

    pub fn get_skill_freq(&self, id: i32) -> i32 {
        for i in 0..16 {
            if self.skill_order[i] == id && id < 35 {
                return self.skills[i].freq;
            }
        }
        0
    }

    pub fn total_skill_sum(&self) -> i32 {
        self.skills.iter().map(|s| s.freq).sum()
    }

    /// 八围 (eight stats) calculation: (atk+def+spd+dex+mag+res+int) + HP/3
    pub fn ba_wei(&self) -> i32 {
        self.atk() + self.def() + self.spd() + self.dex() + self.mag() + self.res() + self.int() + self.hp() / 3
    }
}

/// Initialize val array with 0..255
fn init_val(val: &mut [u8; 256]) {
    for i in 0..256 {
        val[i] = i as u8;
    }
}

/// Shuffle val based on a byte sequence (team name or name body)
fn shuffle_val(val: &mut [u8; 256], bytes: &[u8]) {
    let mut s: u8 = 0;
    for i in 0..256 {
        s = s.wrapping_add(bytes[i % bytes.len()].wrapping_add(val[i]));
        val.swap(i, s as usize);
    }
}

/// Load a name string (name@team format) into val and compute namebase
pub fn load_name(name_in: &str) -> Option<Name> {
    let mut name = Name::new();
    name.raw_name = name_in.to_string();

    // Split at @
    let parts: Vec<&str> = name_in.rsplitn(2, '@').collect();
    let (name_body, team_name) = if parts.len() >= 2 && !parts[1].is_empty() {
        (parts[0], parts[1])
    } else {
        (name_in, name_in)
    };

    let name_bytes = name_body.as_bytes();
    let team_bytes = team_name.as_bytes();

    if name_bytes.is_empty() || team_bytes.is_empty() {
        return None;
    }

    // Build byte sequences with leading 0
    let mut name_seq = vec![0u8];
    name_seq.extend_from_slice(name_bytes);
    let mut team_seq = vec![0u8];
    team_seq.extend_from_slice(team_bytes);

    // Initialize and shuffle val
    init_val(&mut name.val);
    shuffle_val(&mut name.val, &team_seq);
    shuffle_val(&mut name.val, &name_seq);
    shuffle_val(&mut name.val, &name_seq);

    // Compute namebase from val
    let mut bonus_len = 0;
    for i in 0..256 {
        let m = name.val[i].wrapping_mul(181).wrapping_add(160);
        if m >= 89 && m < 217 {
            name.namebase[bonus_len] = m & 63;
            bonus_len += 1;
        }
    }

    // Copy namebase to namebonus
    name.namebonus.copy_from_slice(&name.namebase);

    // Compute properties
    compute_props(&mut name);

    // Compute skills
    compute_skills(&mut name);

    Some(name)
}

/// Compute properties from namebonus (first 32 items)
fn compute_props(name: &mut Name) {
    let mut r: [u8; 32] = [0; 32];
    r.copy_from_slice(&name.namebonus[..32]);

    // HP: sort first 10, sum middle 4 (indices 3..7), +154
    r[0..10].sort();
    name.props[0] = 154 + r[3] as i32 + r[4] as i32 + r[5] as i32 + r[6] as i32;

    // Other 7 stats: each uses 3 items, take median, +36
    let stat_indices: [(usize, usize, usize); 7] = [
        (10, 11, 12), // Atk
        (13, 14, 15), // Def
        (16, 17, 18), // Spd
        (19, 20, 21), // Dex
        (22, 23, 24), // Mag
        (25, 26, 27), // Res
        (28, 29, 30), // Int
    ];

    for (i, &(a, b, c)) in stat_indices.iter().enumerate() {
        let arr = [r[a], r[b], r[c]];
        name.props[i + 1] = median3(arr) as i32 + 36;
    }
}

/// Compute skills from namebonus (items 64..128)
fn compute_skills(name: &mut Name) {
    // Initialize skill IDs 0..39
    let mut skill_ids: [i32; 40] = std::array::from_fn(|i| i as i32);

    // Shuffle skill IDs using val
    let mut rand = Rand::new(0, 0, &name.val);
    let mut s: i32 = 0;
    for _ in 0..2 {
        for j in 0..40 {
            s = (s + rand.next(40) + skill_ids[j]) % 40;
            skill_ids.swap(j as usize, s as usize);
        }
    }

    // Take first 16 skills
    let a = &name.namebonus[64..128];
    let b = &name.namebase[64..128];

    let mut last: i32 = -1;
    let mut j = 0;

    for i in (0..64).step_by(4) {
        let p = a[i..i + 4].iter().cloned().min().unwrap();
        let q = b[i..i + 4].iter().cloned().min().unwrap();

        name.skills[j].id = skill_ids[j];
        name.skill_order[j] = skill_ids[j];

        if p > 10 {
            if skill_ids[j] < 35 {
                name.skills[j].freq = p as i32 - 10;
            }
            if q <= 10 {
                // Skill gained from team bonus, skip tail bonus
                name.skills[j].gained = true;
            } else if skill_ids[j] < 25 {
                last = j as i32;
            }
        }

        j += 1;
    }

    // Last active skill: double frequency
    if last != -1 {
        name.skills[last as usize].gained = true;
        name.skills[last as usize].freq *= 2;
    }

    // Tail bonus for skill slot 14 and 15
    for idx in [14, 15] {
        if name.skills[idx].freq > 0 && !name.skills[idx].gained {
            let bonus = name.namebonus[60 + (idx - 14) * 2].min(name.namebonus[61 + (idx - 14) * 2]) as i32;
            name.skills[idx].freq += bonus.min(name.skills[idx].freq);
            name.skills[idx].gained = true;
        }
    }
}

fn median3(arr: [u8; 3]) -> u8 {
    let mut a = arr;
    a.sort();
    a[1]
}

/// Team bonus calculation: scan namebase and update namebonus
pub fn apply_team_bonus(names: &mut [Name]) {
    let n = names.len();
    for i in 0..n {
        let base_i: [u8; 128] = names[i].namebase;
        for j in (i + 1)..n {
            // i gets bonus from j
            apply_bonus_pair(&mut names[i], &names[j].namebase);
            // j gets bonus from i
            apply_bonus_pair(&mut names[j], &base_i);
        }
    }
    // Recompute props and skills after bonus
    for name in names.iter_mut() {
        compute_props(name);
        compute_skills(name);
    }
}

fn apply_bonus_pair(target: &mut Name, other_base: &[u8; 128]) {
    for i in 7..128 {
        if other_base[i - 1] == target.namebase[i] && other_base[i - 1] > other_base[i] {
            target.namebonus[i] = target.namebonus[i].max(other_base[i]);
        }
    }
}

/// Fast property-only computation (without full skill computation)
/// Used for early pruning in miners
pub fn compute_props_fast(namebase: &[u8; 128]) -> [i32; 8] {
    let mut r: [u8; 32] = [0; 32];
    r.copy_from_slice(&namebase[..32]);

    let mut props = [0i32; 8];

    r[0..10].sort();
    props[0] = 154 + r[3] as i32 + r[4] as i32 + r[5] as i32 + r[6] as i32;

    let stat_indices: [(usize, usize, usize); 7] = [
        (10, 11, 12), (13, 14, 15), (16, 17, 18), (19, 20, 21),
        (22, 23, 24), (25, 26, 27), (28, 29, 30),
    ];

    for (i, &(a, b, c)) in stat_indices.iter().enumerate() {
        let arr = [r[a], r[b], r[c]];
        props[i + 1] = median3(arr) as i32 + 36;
    }

    props
}

pub fn median_u8(a: u8, b: u8, c: u8) -> u8 {
    if a <= b {
        if b <= c { b }
        else if a <= c { c }
        else { a }
    } else {
        if a <= c { a }
        else if b <= c { c }
        else { b }
    }
}
