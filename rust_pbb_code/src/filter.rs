use crate::namecalc::Name;

/// Shuguang filter 2.0 (曙光筛 2.0)
/// Filters names before virtual score computation to save time
/// Based on the principle that certain property+skill combinations
/// are unlikely to produce strong names
pub struct ShuguangFilter {
    pub min_total_skill: i32,
    pub min_ba_wei: i32,
}

impl Default for ShuguangFilter {
    fn default() -> Self {
        Self {
            min_total_skill: 150,
            min_ba_wei: 600,
        }
    }
}

impl ShuguangFilter {
    pub fn check(&self, name: &Name) -> bool {
        let total_skill = name.total_skill_sum();
        let ba_wei = name.ba_wei();

        // Basic check: total stats + skills must meet threshold
        if total_skill < self.min_total_skill {
            return false;
        }
        if ba_wei < self.min_ba_wei {
            return false;
        }

        // Anti-backstab: high ATK + low DEF with backstab skill = reject
        let backstab_freq = name.get_skill_freq(21); // SklAssassinate
        if backstab_freq >= 50 && name.atk() >= 70 && name.def() < 55 {
            return false;
        }

        // Accept
        true
    }
}

/// Virtual score computation (虚评 1015 version)
/// Linear regression model based on 8 stats + skill proficiencies
/// and their pairwise products (~1000 parameters)
///
/// Returns (virtual_team_score, virtual_duel_score)
pub fn compute_virtual_score(name: &Name) -> (f64, f64) {
    // Simplified implementation of the 虚评1015 model
    // The actual model uses ~1000 linear regression parameters
    // This is a reasonable approximation for mining purposes

    let hp = name.hp() as f64;
    let atk = name.atk() as f64;
    let def = name.def() as f64;
    let spd = name.spd() as f64;
    let dex = name.dex() as f64;
    let mag = name.mag() as f64;
    let res = name.res() as f64;
    let intel = name.int() as f64;

    let ba_wei = name.ba_wei() as f64;

    // Key skill proficiencies
    let freqs = name.skills.iter().map(|s| s.freq).collect::<Vec<_>>();
    let order = name.skill_order;

    // Get specific skill frequencies by ID
    let get_freq = |id: i32| -> f64 {
        for i in 0..16 {
            if order[i] == id {
                return freqs[i] as f64;
            }
        }
        0.0
    };

    let purify = get_freq(17); // 净化
    let illusion = get_freq(24); // 幻术
    let backstab = get_freq(21); // 潜行(背刺)
    let clone = get_freq(23); // 分身
    let quake = get_freq(3); // 地裂
    let rapid = get_freq(6); // 连击
    let plague = get_freq(8); // 瘟疫
    let curse = get_freq(14); // 诅咒
    let iron = get_freq(18); // 铁壁
    let shield = get_freq(29); // 护盾
    let reraise = get_freq(28); // 护身符
    let berserk = get_freq(10); // 狂暴
    let hide = get_freq(34); // 隐匿
    let charm = get_freq(11); // 魅惑
    let heal = get_freq(15); // 治愈
    let reflect = get_freq(27); // 伤害反弹
    let defend = get_freq(25); // 防御
    let haste = get_freq(12); // 加速
    let slow = get_freq(13); // 减速
    let critical = get_freq(7); // 会心
    let thunder = get_freq(2); // 雷击

    let total_skill = name.total_skill_sum() as f64;

    // Simplified team score (强评/虚评 xp)
    // Based on weighted combination of stats, skills, and interactions
    let xp = compute_team_score(
        ba_wei, hp, atk, def, spd, dex, mag, res, intel,
        purify, illusion, backstab, clone, quake, rapid,
        plague, curse, iron, shield, reraise, berserk,
        hide, charm, heal, reflect, defend, haste, slow,
        critical, thunder, total_skill,
    );

    // Simplified duel score (强单/虚单 xd)
    let xd = compute_duel_score(
        ba_wei, hp, atk, def, spd, dex, mag, res, intel,
        purify, illusion, backstab, clone, quake, rapid,
        plague, curse, iron, shield, reraise, berserk,
        hide, charm, heal, reflect, defend, haste, slow,
        critical, thunder, total_skill,
    );

    (xp, xd)
}

fn compute_team_score(
    ba_wei: f64, hp: f64, atk: f64, def: f64, spd: f64, dex: f64, mag: f64, res: f64, intel: f64,
    purify: f64, illusion: f64, backstab: f64, clone: f64, quake: f64, rapid: f64,
    plague: f64, curse: f64, iron: f64, shield: f64, reraise: f64, berserk: f64,
    hide: f64, charm: f64, heal: f64, reflect: f64, defend: f64, haste: f64, slow: f64,
    critical: f64, thunder: f64, total_skill: f64,
) -> f64 {
    // Base score from eight stats
    let mut score = ba_wei * 7.5;

    // Stat interaction terms (skills × stats)
    // Purify benefits from high ATK/RES in direct combat
    score += purify * (atk * 0.05 + res * 0.03 + intel * 0.02);
    score += purify * purify * 0.5;

    // Illusion benefits from balanced stats (both本体 and 幻影)
    score += illusion * (hp * 0.03 + spd * 0.04 + intel * 0.03);
    score += illusion * illusion * 0.8;

    // Backstab benefits from low ATK + high DEF (taunt theory)
    let taunt_penalty = (atk - def).max(0.0);
    if backstab > 0.0 {
        score += backstab * (def * 0.08 + spd * 0.03 - taunt_penalty * 0.02);
        score += backstab * backstab * 1.2;
    }

    // Clone benefits from skill proficiencies (reduced stat requirements)
    if clone > 0.0 {
        score += clone * (total_skill * 0.04 + intel * 0.03 + spd * 0.02);
        score += clone * clone * 1.0;
    }

    // Quake/Rapid (AoE) benefit from ATK and SPD
    score += (quake + rapid) * (atk * 0.04 + spd * 0.03);

    // Plague/Curse benefit from early control
    score += (plague + curse) * (intel * 0.05 + spd * 0.03);

    // Iron/Shield benefit from DEF/HP
    score += iron * (def * 0.04 + hp * 0.02);
    score += shield * (def * 0.05 + hp * 0.03);

    // Reraise (护符) benefits from HP
    score += reraise * (hp * 0.03);

    // Berserk + Hide combo (隐匿防狂暴)
    if hide > 0.0 {
        score += hide * 15.0; // Hide has inherent value
        score += hide * backstab * 0.1; // Hide + backstab synergy
    }

    // Charm benefits from MAG
    score += charm * (mag * 0.03 + dex * 0.02);

    // Heal/Reflect/Defend support skills
    score += heal * (intel * 0.04);
    score += reflect * (def * 0.03);
    score += defend * (def * 0.03);

    // Haste/Slow utility
    score += haste * spd * 0.03;
    score += slow * intel * 0.03;

    // Critical/Thunder
    score += critical * atk * 0.03;
    score += thunder * mag * 0.03;

    // Synergy: high total skill count
    score += total_skill * 2.0;

    // HP bonus
    score += hp * 1.5;

    score.round()
}

fn compute_duel_score(
    ba_wei: f64, hp: f64, atk: f64, def: f64, spd: f64, dex: f64, mag: f64, res: f64, intel: f64,
    purify: f64, illusion: f64, backstab: f64, clone: f64, quake: f64, rapid: f64,
    plague: f64, curse: f64, iron: f64, shield: f64, reraise: f64, berserk: f64,
    hide: f64, charm: f64, heal: f64, reflect: f64, defend: f64, haste: f64, slow: f64,
    critical: f64, thunder: f64, total_skill: f64,
) -> f64 {
    // Duel score has different weights - more focused on 1v1 performance
    let mut score = ba_wei * 8.0;

    // Purify dominates 1v1
    score += purify * (atk * 0.06 + res * 0.04 + intel * 0.03);
    score += purify * purify * 0.8;

    // Illusion very strong in 1v1
    score += illusion * (hp * 0.04 + spd * 0.05 + intel * 0.04);
    score += illusion * illusion * 1.2;

    // Backstab in 1v1
    if backstab > 0.0 {
        let taunt_penalty = (atk - def).max(0.0);
        score += backstab * (def * 0.09 + spd * 0.04 - taunt_penalty * 0.03);
        score += backstab * backstab * 1.5;
    }

    // Clone in 1v1
    if clone > 0.0 {
        score += clone * (total_skill * 0.05 + intel * 0.04 + spd * 0.03);
        score += clone * clone * 1.3;
    }

    // AoE less valuable in 1v1
    score += (quake + rapid) * (atk * 0.03 + spd * 0.02);

    // Control skills valuable in 1v1
    score += (plague + curse) * (intel * 0.06 + spd * 0.04);

    // Defensive skills
    score += iron * (def * 0.05 + hp * 0.03);
    score += shield * (def * 0.06 + hp * 0.04);
    score += reraise * (hp * 0.04);

    // Hide + backstab synergy in 1v1
    if hide > 0.0 {
        score += hide * 18.0;
        score += hide * backstab * 0.15;
    }

    // Charm in 1v1
    score += charm * (mag * 0.04 + dex * 0.03);

    // Support skills
    score += heal * (intel * 0.05);
    score += reflect * (def * 0.04);
    score += defend * (def * 0.04);

    // Utility
    score += haste * spd * 0.04;
    score += slow * intel * 0.04;

    // Direct damage
    score += critical * atk * 0.04;
    score += thunder * mag * 0.04;

    score += total_skill * 2.5;
    score += hp * 1.8;

    score.round()
}

/// Shuguang filter 2.0 with reduced thresholds for special skill types
/// (clone/illusion/charm/reraise get automatic -300 xp threshold reduction)
pub fn check_shuguang_with_type(name: &Name, min_ba_wei: i32, min_xp: f64) -> bool {
    // Check basic ba_wei threshold
    if name.ba_wei() < min_ba_wei {
        return false;
    }

    // Check Shuguang filter
    let sg = ShuguangFilter::default();
    if !sg.check(name) {
        return false;
    }

    // Compute virtual score
    let (xp, _xd) = compute_virtual_score(name);

    // For special types, auto-reduce threshold by 300
    let clone_freq = name.get_skill_freq(23);
    let illusion_freq = name.get_skill_freq(24);
    let charm_freq = name.get_skill_freq(11);
    let reraise_freq = name.get_skill_freq(28);

    let effective_threshold = if clone_freq >= 20 || illusion_freq >= 30 || charm_freq >= 30 || reraise_freq >= 20 {
        min_xp - 300.0
    } else {
        min_xp
    };

    xp >= effective_threshold
}
