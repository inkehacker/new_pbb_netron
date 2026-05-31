/// Skill ID constants
pub const SKL_FIRE: i32 = 0;
pub const SKL_ICE: i32 = 1;
pub const SKL_THUNDER: i32 = 2;
pub const SKL_QUAKE: i32 = 3;
pub const SKL_ABSORB: i32 = 4;
pub const SKL_POISON: i32 = 5;
pub const SKL_RAPID: i32 = 6;
pub const SKL_CRITICAL: i32 = 7;
pub const SKL_HALF: i32 = 8;
pub const SKL_EXCHANGE: i32 = 9;
pub const SKL_BERSERK: i32 = 10;
pub const SKL_CHARM: i32 = 11;
pub const SKL_HASTE: i32 = 12;
pub const SKL_SLOW: i32 = 13;
pub const SKL_CURSE: i32 = 14;
pub const SKL_HEAL: i32 = 15;
pub const SKL_REVIVE: i32 = 16;
pub const SKL_DISPERSE: i32 = 17;
pub const SKL_IRON: i32 = 18;
pub const SKL_CHARGE: i32 = 19;
pub const SKL_ACCUMULATE: i32 = 20;
pub const SKL_ASSASSINATE: i32 = 21;
pub const SKL_SUMMON: i32 = 22;
pub const SKL_CLONE: i32 = 23;
pub const SKL_SHADOW: i32 = 24;
pub const SKL_DEFEND: i32 = 25;
pub const SKL_PROTECT: i32 = 26;
pub const SKL_REFLECT: i32 = 27;
pub const SKL_RERAISE: i32 = 28;
pub const SKL_SHIELD: i32 = 29;
pub const SKL_COUNTER: i32 = 30;
pub const SKL_MERGE: i32 = 31;
pub const SKL_ZOMBIE: i32 = 32;
pub const SKL_UPGRADE: i32 = 33;
pub const SKL_HIDE: i32 = 34;

pub fn skill_name(id: i32) -> &'static str {
    match id {
        0 => "火球术", 1 => "冰冻术", 2 => "雷击术", 3 => "地裂术", 4 => "吸血攻击",
        5 => "投毒", 6 => "连击", 7 => "会心一击", 8 => "瘟疫", 9 => "生命之轮",
        10 => "狂暴术", 11 => "魅惑", 12 => "加速术", 13 => "减速术", 14 => "诅咒",
        15 => "治愈魔法", 16 => "苏生术", 17 => "净化", 18 => "铁壁", 19 => "蓄力",
        20 => "聚气", 21 => "潜行", 22 => "血祭", 23 => "分身", 24 => "幻术",
        25 => "防御", 26 => "守护", 27 => "伤害反弹", 28 => "护身符", 29 => "护盾",
        30 => "反击", 31 => "吞噬", 32 => "召唤亡灵", 33 => "垂死抗争", 34 => "隐匿",
        _ => "(空技能)",
    }
}

pub fn skill_id(name: &str) -> Option<i32> {
    match name {
        "火球术" | "SklFire" => Some(0),
        "冰冻术" | "SklIce" => Some(1),
        "雷击术" | "SklThunder" => Some(2),
        "地裂术" | "SklQuake" => Some(3),
        "吸血攻击" | "SklAbsorb" => Some(4),
        "投毒" | "SklPoison" => Some(5),
        "连击" | "SklRapid" => Some(6),
        "会心一击" | "SklCritical" => Some(7),
        "瘟疫" | "SklHalf" => Some(8),
        "生命之轮" | "SklExchange" => Some(9),
        "狂暴术" | "SklBerserk" => Some(10),
        "魅惑" | "SklCharm" => Some(11),
        "加速术" | "SklHaste" => Some(12),
        "减速术" | "SklSlow" => Some(13),
        "诅咒" | "SklCurse" => Some(14),
        "治愈魔法" | "SklHeal" => Some(15),
        "苏生术" | "SklRevive" => Some(16),
        "净化" | "SklDisperse" => Some(17),
        "铁壁" | "SklIron" => Some(18),
        "蓄力" | "SklCharge" => Some(19),
        "聚气" | "SklAccumulate" => Some(20),
        "潜行" | "SklAssassinate" => Some(21),
        "血祭" | "SklSummon" => Some(22),
        "分身" | "SklClone" => Some(23),
        "幻术" | "SklShadow" => Some(24),
        "防御" | "SklDefend" => Some(25),
        "守护" | "SklProtect" => Some(26),
        "伤害反弹" | "SklReflect" => Some(27),
        "护身符" | "SklReraise" => Some(28),
        "护盾" | "SklShield" => Some(29),
        "反击" | "SklCounter" => Some(30),
        "吞噬" | "SklMerge" => Some(31),
        "召唤亡灵" | "SklZombie" => Some(32),
        "垂死抗争" | "SklUpgrade" => Some(33),
        "隐匿" | "SklHide" => Some(34),
        _ => None,
    }
}

/// Get the slot index for a given skill ID
pub fn get_slot_for_skill(name: &super::namecalc::Name, skill_id: i32) -> Option<usize> {
    for i in 0..16 {
        if name.skill_order[i] == skill_id {
            return Some(i);
        }
    }
    None
}
