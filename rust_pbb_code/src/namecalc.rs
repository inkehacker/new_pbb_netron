pub const N: usize = 256;

#[derive(Clone, Copy, Debug, Default)]
pub struct SkillSlot {
    pub id: u8,
    pub freq: u8,
}

pub struct Name {
    pub val: [u8; N],
    pub name_base: [u8; 128],
    pub skill: [SkillSlot; 16],
    pub raw_skill: [u8; 40],
    pub v_sum: u32,
}

impl Name {
    pub fn new() -> Self {
        Name {
            val: [0; N],
            name_base: [0; 128],
            skill: [SkillSlot::default(); 16],
            raw_skill: [0; 40],
            v_sum: 0,
        }
    }
}

#[inline(always)]
pub fn median3(a: u8, b: u8, c: u8) -> u8 {
    if a < b {
        if a < c { if b < c { b } else { c } }
        else { a }
    } else {
        if b < c { if a < c { a } else { c } }
        else { b }
    }
}

#[inline(always)]
pub fn compute_props(name_base: &[u8; 128]) -> [u32; 8] {
    let mut props = [0u32; 8];
    props[0] = median3(name_base[10], name_base[11], name_base[12]) as u32;
    props[1] = median3(name_base[13], name_base[14], name_base[15]) as u32;
    props[2] = median3(name_base[16], name_base[17], name_base[18]) as u32;
    props[3] = median3(name_base[19], name_base[20], name_base[21]) as u32;
    props[4] = median3(name_base[22], name_base[23], name_base[24]) as u32;
    props[5] = median3(name_base[25], name_base[26], name_base[27]) as u32;
    props[6] = median3(name_base[28], name_base[29], name_base[30]) as u32;
    props[7] = (154u32 + name_base[3] as u32 + name_base[4] as u32
        + name_base[5] as u32 + name_base[6] as u32) / 3;
    props
}

pub fn load_team(val_base: &mut [u8; N], team_bytes: &[i8]) {
    let t_len = team_bytes.len() + 1;
    for i in 0..N { val_base[i] = i as u8; }
    let mut s: u8 = 0;
    for i in 0..N {
        if i % t_len != 0 {
            s = s.wrapping_add(team_bytes[i % t_len - 1] as u8);
        }
        s = s.wrapping_add(val_base[i]);
        val_base.swap(i, s as usize);
    }
}

#[inline(always)]
pub fn load_name_fast(
    val_base2: &[u8; N],
    pre_len: usize,
    total_len: usize,
    name_bytes: &[u8],
    name_base: &mut [u8; 128],
) -> Option<u32> {
    let mut val = *val_base2;

    let mut j_prefix: isize = total_len as isize;
    let mut s_prefix: u8 = 0;
    for _i in 0..pre_len {
        let jv = j_prefix as usize;
        let bv = if jv <= total_len && jv < name_bytes.len() { name_bytes[jv] } else { 0 };
        s_prefix = s_prefix.wrapping_add(bv).wrapping_add(val_base2[_i]);
        j_prefix += 1;
        if j_prefix == total_len as isize { j_prefix = -1; }
    }

    let mut j_first = j_prefix;
    if j_first < 0 { j_first = 0; }
    if pre_len == 0 { j_first = total_len as isize; }

    let mut s = s_prefix;
    for i_idx in pre_len..N {
        let j_usize = j_first as usize;
        let bv = if j_usize <= total_len && j_usize < name_bytes.len() { name_bytes[j_usize] } else { 0 };
        s = s.wrapping_add(bv).wrapping_add(val[i_idx]);
        val.swap(i_idx, s as usize);
        j_first += 1;
        if j_first == total_len as isize { j_first = -1; }
    }

    s = 0;
    let mut j_second = total_len as isize;
    for i in 0..N {
        let j_usize = j_second as usize;
        let bv = if j_usize <= total_len && j_usize < name_bytes.len() { name_bytes[j_usize] } else { 0 };
        s = s.wrapping_add(bv).wrapping_add(val[i]);
        val.swap(i, s as usize);
        j_second += 1;
        if j_second == total_len as isize { j_second = -1; }
    }

    let mut q_len: isize = -1;

    for i in (0..96).step_by(8) {
        let u0 = val[i].wrapping_mul(181).wrapping_add(160);
        let u1 = val[i+1].wrapping_mul(181).wrapping_add(160);
        let u2 = val[i+2].wrapping_mul(181).wrapping_add(160);
        let u3 = val[i+3].wrapping_mul(181).wrapping_add(160);
        let u4 = val[i+4].wrapping_mul(181).wrapping_add(160);
        let u5 = val[i+5].wrapping_mul(181).wrapping_add(160);
        let u6 = val[i+6].wrapping_mul(181).wrapping_add(160);
        let u7 = val[i+7].wrapping_mul(181).wrapping_add(160);

        if u0 >= 89 && u0 < 217 { q_len += 1; name_base[q_len as usize] = u0 & 63; }
        if u1 >= 89 && u1 < 217 { q_len += 1; name_base[q_len as usize] = u1 & 63; }
        if u2 >= 89 && u2 < 217 { q_len += 1; name_base[q_len as usize] = u2 & 63; }
        if u3 >= 89 && u3 < 217 { q_len += 1; name_base[q_len as usize] = u3 & 63; }
        if u4 >= 89 && u4 < 217 { q_len += 1; name_base[q_len as usize] = u4 & 63; }
        if u5 >= 89 && u5 < 217 { q_len += 1; name_base[q_len as usize] = u5 & 63; }
        if u6 >= 89 && u6 < 217 { q_len += 1; name_base[q_len as usize] = u6 & 63; }
        if u7 >= 89 && u7 < 217 { q_len += 1; name_base[q_len as usize] = u7 & 63; }
        if q_len >= 30 { break; }
    }

    if q_len < 30 {
        for i in (96..N).step_by(8) {
            let u0 = val[i].wrapping_mul(181).wrapping_add(160);
            let u1 = val[i+1].wrapping_mul(181).wrapping_add(160);
            let u2 = val[i+2].wrapping_mul(181).wrapping_add(160);
            let u3 = val[i+3].wrapping_mul(181).wrapping_add(160);
            let u4 = val[i+4].wrapping_mul(181).wrapping_add(160);
            let u5 = val[i+5].wrapping_mul(181).wrapping_add(160);
            let u6 = val[i+6].wrapping_mul(181).wrapping_add(160);
            let u7 = val[i+7].wrapping_mul(181).wrapping_add(160);

            if u0 >= 89 && u0 < 217 { q_len += 1; name_base[q_len as usize] = u0 & 63; }
            if u1 >= 89 && u1 < 217 { q_len += 1; name_base[q_len as usize] = u1 & 63; }
            if u2 >= 89 && u2 < 217 { q_len += 1; name_base[q_len as usize] = u2 & 63; }
            if u3 >= 89 && u3 < 217 { q_len += 1; name_base[q_len as usize] = u3 & 63; }
            if u4 >= 89 && u4 < 217 { q_len += 1; name_base[q_len as usize] = u4 & 63; }
            if u5 >= 89 && u5 < 217 { q_len += 1; name_base[q_len as usize] = u5 & 63; }
            if u6 >= 89 && u6 < 217 { q_len += 1; name_base[q_len as usize] = u6 & 63; }
            if u7 >= 89 && u7 < 217 { q_len += 1; name_base[q_len as usize] = u7 & 63; }
            if q_len >= 30 { break; }
        }
    }

    if q_len < 30 { return None; }

    let mut v: u32 = 0;
    v += median3(name_base[28], name_base[29], name_base[30]) as u32;
    if v < 24 { return None; }
    v += median3(name_base[13], name_base[14], name_base[15]) as u32;
    v += median3(name_base[16], name_base[17], name_base[18]) as u32;
    v += median3(name_base[25], name_base[26], name_base[27]) as u32;
    if v < 165 { return None; }
    v += median3(name_base[10], name_base[11], name_base[12]) as u32;
    v += median3(name_base[19], name_base[20], name_base[21]) as u32;
    v += median3(name_base[22], name_base[23], name_base[24]) as u32;
    if v < 250 { return None; }

    let mut sorted10 = [0u8; 10];
    sorted10.copy_from_slice(&name_base[0..10]);
    sorted10.sort_unstable();
    v += (154u32 + sorted10[3] as u32 + sorted10[4] as u32 + sorted10[5] as u32 + sorted10[6] as u32) / 3;

    Some(v)
}

#[inline(always)]
pub fn fill_full_namebase(val: &[u8; N], name_base: &mut [u8; 128]) -> usize {
    let mut q_len: isize = -1;
    for i in 0..N {
        let u = val[i].wrapping_mul(181).wrapping_add(160);
        if u >= 89 && u < 217 {
            q_len += 1;
            if q_len < 128 {
                name_base[q_len as usize] = u & 63;
            }
        }
    }
    q_len as usize
}

#[inline(always)]
pub fn calc_skills(val: &mut [u8; N], name_base: &mut [u8; 128], skill: &mut [SkillSlot; 16]) {
    let mut s: u8 = 0;
    for i in 0..N {
        let j = i % 128;
        s = s.wrapping_add(name_base[j]).wrapping_add(val[i]);
        val.swap(i, s as usize);
    }

    for i in 0..128 { name_base[i] = 0; }

    let mut n_len: usize = 0;
    for i in 0..40 {
        loop {
            if n_len >= 40 { break; }
            let v = val[n_len];
            n_len += 1;
            if v != 0 && v < 36 {
                name_base[i] = v;
                break;
            }
        }
    }

    let mut freq = [0u8; 35];
    let mut skill_count: usize = 0;
    for i in 0..40 {
        let id = name_base[i] as usize;
        if id > 0 {
            freq[id] += 1;
            if freq[id] == 1 { skill_count += 1; }
        }
    }

    for i in 0..skill_count.min(16) {
        let id = name_base[i];
        let freq_val = freq[id as usize] * 100 / skill_count as u8;
        skill[i] = SkillSlot { id, freq: freq_val };
    }

    if skill_count > 0 && skill_count <= 16 {
        let last_idx = skill_count - 1;
        if last_idx < 16 {
            skill[last_idx].freq *= 2;
        }
    }

    if skill_count < 16 {
        for i in skill_count..16 {
            skill[i] = SkillSlot { id: 0, freq: 0 };
        }
    }
}

#[inline(always)]
pub fn loading_name(val: &mut [u8; N], val_base: &[u8; N], name_bytes: &[u8]) -> [u32; 8] {
    val.copy_from_slice(val_base);
    let name_len = name_bytes.len();
    let mut s: u8 = 0;
    for i in 0..N {
        let j = i % name_len;
        s = s.wrapping_add(name_bytes[j]).wrapping_add(val[i]);
        val.swap(i, s as usize);
    }
    s = 0;
    for i in 0..N {
        let j = i % name_len;
        s = s.wrapping_add(name_bytes[j]).wrapping_add(val[i]);
        val.swap(i, s as usize);
    }
    let mut name_base = [0u8; 128];
    let mut q_len: usize = 0;
    for i in 0..N {
        let u = val[i].wrapping_mul(181).wrapping_add(160);
        if u >= 89 && u < 217 {
            name_base[q_len] = u & 63;
            q_len += 1;
            if q_len >= 128 { break; }
        }
    }
    compute_props(&name_base)
}
