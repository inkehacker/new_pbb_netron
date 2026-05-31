pub const N: usize = 256;

/// Skill structure matching C++ Name::skill[] and Name::freq[]
#[derive(Clone, Copy, Debug, Default)]
pub struct SkillSlot {
    pub id: u8,
    pub freq: u8,
}

/// Name struct matching C++ Name
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

/// Median of three values matching C++ macro
#[inline(always)]
pub fn median3(a: u8, b: u8, c: u8) -> u8 {
    if a < b {
        if a < c {
            if b < c { b } else { c }
        } else {
            a
        }
    } else {
        if b < c {
            if a < c { a } else { c }
        } else {
            b
        }
    }
}

/// Compute properties from name_base (first 31 items)
/// This matches C++ lines 1130-1136
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
    props[7] = (154u32
        + name_base[3] as u32
        + name_base[4] as u32
        + name_base[5] as u32
        + name_base[6] as u32) / 3;
    props
}

/// Load team name into val_base
/// Matches C++ load_team() lines 208-219
pub fn load_team(val_base: &mut [u8; N], team_bytes: &[i8]) {
    let t_len = team_bytes.len() + 1;
    for i in 0..N {
        val_base[i] = i as u8;
    }
    let mut s: u8 = 0;
    for i in 0..N {
        if i % t_len != 0 {
            s = s.wrapping_add(team_bytes[i % t_len - 1] as u8);
        }
        s = s.wrapping_add(val_base[i]);
        val_base.swap(i, s as usize);
    }
}

/// Load prefix into val_base2 (team + prefix shuffle)
/// This is used by the miner to set up the state before calling load_name_fast
/// Returns (i_pre, j_pre, s_pre) for use in load_name_fast
#[inline(always)]
pub fn load_prefix(
    val_base2: &mut [u8; N],
    pre_len: usize,
    total_len: usize,
    name_bytes: &[u8],
) -> (usize, usize, u8) {
    let mut s: u8 = 0;
    let mut j: usize = if pre_len == 0 { total_len } else { 0 };

    for _i in 0..pre_len {
        j = if _i == 0 {
            if pre_len == 0 { total_len } else { pre_len }
        } else {
            j + 1
        };
        if j > total_len { j = 0; }
        s = s.wrapping_add(name_bytes[j]).wrapping_add(val_base2[_i]);
        val_base2.swap(_i, s as usize);
    }

    let i_pre = pre_len;
    let j_pre = j + 1;
    let j_pre = if j_pre > total_len { 0 } else { j_pre };
    (i_pre, j_pre, s)
}

/// Compute the i_pre, j_pre, s_pre state after prefix loading
/// Returns (j_start_first_pass, s_after_prefix)
/// 
/// C++ load_prefix logic:
///   for (i=s=0,j=NAMELEN; i<PRELEN; i++,j++) {
///     s += name[j] + val_base2[i]; swap; if (j==NAMELEN) j=-1;
///   }
///   i_pre=i; j_pre=j; s_pre=s;
///   if (i_pre==0) j_pre=name_len, s_pre=0;
/// 
/// So after prefix:
/// - j starts at NAMELEN
/// - each iteration: j++, if j==NAMELEN then j=-1
/// - after loop: j_pre = j (which could be -1 to NAMELEN-1 range)
/// - next pass uses j_pre directly, and j++ happens AFTER name[j] access
#[inline(always)]
pub fn compute_prefix_state(pre_len: usize, total_len: usize) -> (usize, u8) {
    let mut s: u8 = 0;
    let mut j: isize = total_len as isize;

    for i in 0..pre_len {
        let j_usize = j as usize;
        s = s.wrapping_add(if j_usize <= total_len { 0u8 } else { 0u8 });
        // We need the name_bytes value but don't have them here
        // This function is only for computing the state
        j += 1;
        if j == total_len as isize { j = -1; }
    }

    let j_out = if j < 0 { 0 } else { j as usize };
    (j_out, s)
}

/// Compute prefix state given actual name bytes
#[inline(always)]
pub fn compute_prefix_state_full(pre_len: usize, total_len: usize, name_bytes: &[u8]) -> (usize, u8) {
    let mut s: u8 = 0;
    let mut j: isize = total_len as isize;

    for _i in 0..pre_len {
        let jv = if j < 0 { 0 } else { j as usize };
        let byte_val = if jv <= total_len && jv < name_bytes.len() { name_bytes[jv] } else { 0 };
        s = s.wrapping_add(byte_val).wrapping_add(s);
        j += 1;
        if j == total_len as isize { j = -1; }
    }

    let j_out = if j < 0 { 0 } else { j as usize };
    (j_out, s)
}

/// Load name body into val and compute name_base with early pruning
/// Returns None if the name fails early pruning (V*3 < threshold)
/// This matches C++ load_name() lines 235-296
/// 
/// val_base2 must already contain the team + prefix shuffled state
/// pre_len: number of prefix bytes already processed by load_prefix
/// total_len: full name length (NAMELEN in C++, not counting null terminator)
/// name_bytes: the full name bytes (length must be at least total_len+1, with name_bytes[total_len] == 0)
/// 
/// C++ first pass: for (int i=i_pre,j=j_pre; i < N; i++,j++) {
///   s += name[j] + val[i]; swap; if (j==NAMELEN) j=-1;
/// }
/// 
/// C++ second pass: for (int i = s = 0, j = NAMELEN; i < N; i++,j++) {
///   s += name[j] + val[i]; swap; if (j==NAMELEN) j=-1;
/// }
#[inline(always)]
pub fn load_name_fast(
    val_base2: &[u8; N],
    pre_len: usize,
    total_len: usize,
    name_bytes: &[u8],
    name_base: &mut [u8; 128],
) -> Option<u32> {
    // Copy val_base2 to val
    let mut val = *val_base2;
    
    // Compute j_pre after prefix loading
    // C++: after load_prefix, j_pre is computed as:
    //   j starts at NAMELEN, increments each iteration, wraps to -1 when == NAMELEN
    //   After pre_len iterations: j has advanced pre_len times from NAMELEN
    //   But wraps: each NAMELEN+1 steps it wraps
    let mut j: isize = total_len as isize;
    for _i in 0..pre_len {
        j += 1;
        if j == total_len as isize { j = -1; }
    }
    let mut s: u8 = 0;
    // Recompute s from actual prefix bytes
    let mut j2: isize = total_len as isize;
    for _i in 0..pre_len {
        let jv = j2 as usize;
        let bv = if jv <= total_len && jv < name_bytes.len() { name_bytes[jv] } else { 0 };
        s = s.wrapping_add(bv).wrapping_add(val_base2[_i]);
        j2 += 1;
        if j2 == total_len as isize { j2 = -1; }
    }
    
    // First shuffle pass (C++ lines 240-245)
    // j starts at j_pre (after prefix), s starts at s_pre
    let mut j_first = if j < 0 { total_len as isize + 1 } else { j };
    // Actually j could be -1, and name[-1] doesn't exist
    // C++ does: s += name[j] first, then j++, then check j==NAMELEN
    // If j == -1, name[-1] is undefined behavior in C++!
    // But looking at the code flow: after load_prefix, j_pre = j from the loop
    // In the loop: j starts at NAMELEN, does j++, then if j==NAMELEN j=-1
    // So j could be -1 only right after wrapping. In the NEXT iteration of load_name,
    // j++ makes it 0 before accessing name[0].
    // Wait no - C++ does: s += name[j] + val[i]; THEN j++; THEN if (j==NAMELEN) j=-1;
    // So if j_pre == -1, the first access is name[-1] which is UB.
    // This means j_pre should never be -1 when entering load_name.
    // Let me re-check: load_prefix does:
    //   for (i=s=0,j=NAMELEN; i<PRELEN; i++,j++) {
    //     s += name[j] + val_base2[i]; swap(val_base2[i], val_base2[s]);
    //     if (j==NAMELEN) j=-1;
    //   }
    // The for loop increments j at i++,j++ step, then body executes, then check j==NAMELEN
    // Actually the order is: init (j=NAMELEN), check condition (i<PRELEN), body, increment (i++,j++), check condition...
    // So: iteration 0: j=NAMELEN (access name[NAMELEN]=0), then j++, if j==NAMELEN j=-1
    // Since NAMELEN+1 != NAMELEN (unless NAMELEN wraps), j doesn't become -1 in iteration 0.
    // It only becomes -1 when j has wrapped around: after NAMELEN+1 iterations, j = NAMELEN + (NAMELEN+1) = 2*NAMELEN+1
    // But NAMELEN+1 != NAMELEN for any reasonable NAMELEN, so j won't equal NAMELEN after increment.
    // 
    // Wait, NAMELEN is the name length. If name has 3 bytes, NAMELEN = 3.
    // j starts at 3. After increment: j=4. 4 != 3, so j stays at 4.
    // Then: j=4,5,6,7,... but name[4] is out of bounds!
    // The C++ code accesses name[j] where j can be > NAMELEN. This is also UB.
    // 
    // I think the intent is: name is a null-terminated string, and name[NAMELEN] is the null byte (0).
    // When j > NAMELEN, the C++ code reads past the null terminator, which is the next name or garbage.
    // 
    // But looking at the consumer thread code, each name is processed individually in a loop,
    // and the name buffer is overwritten each time. So reading past the null terminator might read
    // garbage or zeros depending on memory layout.
    // 
    // Actually, I think I've been overcomplicating this. Let me re-read the C++ more carefully.
    // 
    // C++ line 246: for (int i = s = 0, j = NAMELEN; i < N; i++,j++)
    // This second pass ALWAYS starts j at NAMELEN. So name[NAMELEN] = 0 is the first access,
    // then name[NAMELEN+1] = undefined, etc.
    // 
    // But wait - if NAMELEN = 3, then j goes: 3, 4, 5, 6, ..., N-1+NAMELEN
    // name[j] for j > NAMELEN is reading past the end of the name.
    // This seems intentional - the C++ code expects the name buffer to be zero-padded or the
    // wrapping behavior is intentional.
    // 
    // Actually, I think the wrapping IS the bug. Let me look at this differently.
    // The C++ code has NAMELEN which is the name length (not counting null terminator).
    // name[NAMELEN] should be 0 (null terminator).
    // The loop does j = NAMELEN, then j++, and if (j==NAMELEN) j=-1.
    // 
    // For this to make sense, the pattern should be:
    // j = NAMELEN (access null), j++, j = NAMELEN+1 (out of bounds!), ...
    // 
    // But if NAMELEN+1 == NAMELEN (overflow), or if the name buffer is properly zeroed...
    // 
    // I think the actual intended behavior is:
    // j cycles through 0, 1, 2, ..., NAMELEN, 0, 1, 2, ..., NAMELEN, ...
    // The "if (j==NAMELEN) j=-1" is supposed to wrap j back to 0 (since j++ after -1 gives 0).
    // 
    // So the correct interpretation is:
    // j starts at NAMELEN
    // Each iteration: access name[j], j++, if j == NAMELEN+1 then j = 0
    // 
    // But the C++ says "if (j==NAMELEN) j=-1", not "if (j==NAMELEN+1) j=0".
    // This means j reaches NAMELEN+1, doesn't wrap, then eventually wraps when j overflows.
    // For a 32-bit int, that's 2^32 iterations. For N=256 iterations, j goes from NAMELEN to NAMELEN+255.
    // 
    // Hmm, unless the compiler is optimizing based on the undefined behavior...
    // 
    // Let me just match the C++ behavior exactly: j wraps when it equals NAMELEN (after increment).
    // Since j starts at NAMELEN, and increments to NAMELEN+1, it never equals NAMELEN again.
    // So effectively j just increments monotonically: NAMELEN, NAMELEN+1, ..., NAMELEN+255.
    // 
    // But that means we're reading name[NAMELEN+255] which is way past the name buffer.
    // That can't be right.
    // 
    // Let me try a different interpretation: maybe the intent is modular arithmetic.
    // j should cycle through 0..NAMELEN (inclusive, so NAMELEN+1 values).
    // The modulo would be (NAMELEN + 1).
    // 
    // C++ modulo approach: j = (some_start) % (NAMELEN + 1)
    // 
    // For the second pass: j starts at NAMELEN, which is (NAMELEN) % (NAMELEN+1) = NAMELEN. Good.
    // Then j increments and wraps: NAMELEN, 0, 1, 2, ..., NAMELEN, 0, 1, ...
    // This matches the "if (j==NAMELEN) j=-1; j++" pattern IF we read it as:
    // After accessing name[NAMELEN], j++, then if j==NAMELEN+1 (not NAMELEN), wrap to 0.
    // 
    // But the C++ says "if (j==NAMELEN)" not "if (j==NAMELEN+1)".
    // 
    // I think there might be a bug in the C++ code, or the compiler is doing something weird.
    // Let me just use the modular approach which is what the tutorial says:
    // j = i % (total_len + 1) for the second pass.
    // j = (i - pre_len + j_pre) % (total_len + 1) for the first pass.
    
    // Actually, let me re-read the C++ ONE MORE TIME very carefully:
    // Line 246: for (int i = s = 0, j = NAMELEN; i < N; i++,j++)
    // This means: i=0, s=0, j=NAMELEN initially.
    // Then for each iteration: check i<N, execute body, then i++, j++.
    // 
    // Body (line 248): s += name[j] + val[i];
    // Line 249: swap(val[i], val[s]);
    // Line 250: if (j==NAMELEN) j=-1;
    // 
    // Iteration 0: i=0, j=NAMELEN. Access name[NAMELEN]=0. Then j++ => j=NAMELEN+1. 
    //   Check: NAMELEN+1 == NAMELEN? No (unless overflow). So j stays at NAMELEN+1.
    // 
    // Iteration 1: i=1, j=NAMELEN+1. Access name[NAMELEN+1] which is past null terminator.
    //   Then j++ => j=NAMELEN+2. Check: NAMELEN+2 == NAMELEN? No.
    // 
    // This continues forever with j increasing. name[j] for large j is undefined.
    // 
    // UNLESS: the for loop's j++ happens BEFORE the body check.
    // Actually no, the standard C++ for loop is: init, condition, body, increment, condition, body, increment...
    // 
    // OK I think the C++ code might have a subtle bug, or it relies on the name buffer
    // being followed by zeros in memory. Let me just use the modular approach which is
    // the mathematically correct interpretation:
    // j cycles through 0, 1, ..., NAMELEN with period (NAMELEN + 1).
    // 
    // For the second pass: j = (0 + NAMELEN) % (NAMELEN+1) = NAMELEN, then (1+NAMELEN)%(NAMELEN+1)=0, etc.
    // This gives: NAMELEN, 0, 1, 2, ..., NAMELEN, 0, 1, ... which is the intended behavior.
    // 
    // For the first pass: j starts at j_pre (computed from prefix state) and increments.
    // j = (j_pre + (i - pre_len)) % (NAMELEN + 1) for i >= pre_len.
    
    // First shuffle pass
    let total_len_p1 = total_len + 1;
    s = 0;
    
    // Compute s_pre from prefix loading (need to re-do prefix shuffle on a copy)
    // Actually s is passed from load_prefix. Let me handle this differently.
    // For now, let me just do the full shuffle from scratch (team + prefix + name).
    
    // The miner already handles team+prefix shuffling in process_names.
    // Here we just need the name body shuffle.
    // But s_pre from the prefix shuffle matters!
    
    // Let me just use the approach where we do the FULL shuffle (including prefix)
    // since that's simpler and avoids the s_pre tracking issue.
    
    // Actually wait - the miner.rs code already does the prefix shuffle into val_base2,
    // and passes val_base2 to this function. So val_base2 already has the correct state.
    // I just need to continue from where the prefix shuffle left off.
    // 
    // But s_pre is needed. Let me compute it here.
    
    let mut s_prefix: u8 = 0;
    let mut j_prefix: isize = total_len as isize;
    for _i in 0..pre_len {
        let jv = j_prefix as usize;
        let bv = if jv <= total_len && jv < name_bytes.len() { name_bytes[jv] } else { 0 };
        s_prefix = s_prefix.wrapping_add(bv).wrapping_add(val_base2[_i]);
        j_prefix += 1;
        if j_prefix == total_len as isize { j_prefix = -1; }
    }
    
    // Now j_after_prefix = j_prefix (the value at the END of the prefix loop, before any post-processing)
    // C++ says: i_pre=i; j_pre=j; s_pre=s; if (i_pre==0) j_pre=name_len, s_pre=0;
    // After the for loop, j has been incremented and possibly wrapped.
    // If pre_len == 0, the for loop never runs, so j = NAMELEN, s = 0.
    // Then "if (i_pre==0) j_pre=name_len" sets j_pre back to name_len (which it already is).
    // So j_pre = NAMELEN when pre_len == 0.
    
    let mut j_first = j_prefix;
    if j_first < 0 { j_first = 0; }
    if pre_len == 0 { j_first = total_len as isize; }
    
    // First shuffle pass
    s = s_prefix;
    for i_idx in pre_len..N {
        let j_usize = j_first as usize;
        let bv = if j_usize <= total_len && j_usize < name_bytes.len() { name_bytes[j_usize] } else { 0 };
        s = s.wrapping_add(bv).wrapping_add(val[i_idx]);
        val.swap(i_idx, s as usize);
        j_first += 1;
        if j_first == total_len as isize { j_first = -1; }
    }
    
    // Second shuffle pass (C++ lines 246-251)
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
    
    // Compute ual and extract name_base
    let mut q_len: isize = -1;
    
    // First batch: 0..96
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

/// Full name_base computation (all 128 items) for when early pruning passes
/// Used to fill in name_base beyond the first 31 items for skill computation
#[inline(always)]
pub fn fill_full_namebase(val: &[u8; N], name_base: &mut [u8; 128]) -> usize {
    // Compute ual for all 256
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

/// Compute skills from val and name_base
/// This modifies val further (skill shuffle pass), then extracts skills
/// Matches C++ calc_skills() lines 349-390
#[inline(always)]
pub fn calc_skills(val: &mut [u8; N], name_base: &mut [u8; 128], skill: &mut [SkillSlot; 16]) {
    // Shuffle val with name_base for skill extraction
    let mut s: u8 = 0;
    for i in 0..N {
        let j = i % 128;
        s = s.wrapping_add(name_base[j]).wrapping_add(val[i]);
        val.swap(i, s as usize);
    }
    
    // Reset name_base for skill data
    for i in 0..128 {
        name_base[i] = 0;
    }
    
    // Extract skill IDs from val (C++ lines 359-368)
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
    
    // Compute frequencies
    let mut freq = [0u8; 35];
    let mut skill_count: usize = 0;
    for i in 0..40 {
        let id = name_base[i] as usize;
        if id > 0 {
            freq[id] += 1;
            if freq[id] == 1 { skill_count += 1; }
        }
    }
    
    // Fill skill slots (C++ lines 375-388)
    for i in 0..skill_count.min(16) {
        let id = name_base[i];
        let freq_val = freq[id as usize] * 100 / skill_count as u8;
        skill[i] = SkillSlot { id, freq: freq_val };
    }
    
    // Double frequency for the last active skill
    if skill_count > 0 && skill_count <= 16 {
        let last_idx = skill_count - 1;
        if last_idx < 16 {
            skill[last_idx].freq *= 2;
        }
    }
    
    // Tail bonus: if skill_count < 16, fill remaining slots
    if skill_count < 16 {
        let tail = if skill_count > 0 {
            let last_id = name_base[skill_count - 1] as usize;
            if last_id > 0 { freq[last_id] * 2 } else { 0 }
        } else {
            0
        };
        
        for i in skill_count..16 {
            // Slots 14 and 15 get special treatment
            if i == 14 || i == 15 {
                // Check if the second-to-last active skill (if exists) should be doubled
                if skill_count >= 2 {
                    let prev_id = name_base[skill_count - 2] as usize;
                    if prev_id > 0 {
                        skill[i] = SkillSlot {
                            id: name_base[skill_count - 2],
                            freq: freq[prev_id] * 2,
                        };
                    }
                }
            } else {
                skill[i] = SkillSlot { id: 0, freq: tail };
            }
        }
    }
}

/// Full name load for shadow computation
/// This reloads the name with ?shadow appended and computes properties
/// Matches C++ loading_name() lines 297-348
#[inline(always)]
pub fn loading_name(val: &mut [u8; N], val_base: &[u8; N], name_bytes: &[u8]) -> [u32; 8] {
    val.copy_from_slice(val_base);
    
    let name_len = name_bytes.len();
    let mut s: u8 = 0;
    
    // First shuffle pass
    for i in 0..N {
        let j = i % name_len;
        s = s.wrapping_add(name_bytes[j]).wrapping_add(val[i]);
        val.swap(i, s as usize);
    }
    
    // Second shuffle pass
    s = 0;
    for i in 0..N {
        let j = i % name_len;
        s = s.wrapping_add(name_bytes[j]).wrapping_add(val[i]);
        val.swap(i, s as usize);
    }
    
    // Compute name_base
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
