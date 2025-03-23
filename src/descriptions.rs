// region:  --- Resources

pub const LIFESTYLE: &str = r#"
Lifestyles represent the standard of living a character maintains, reflecting their income, social status, and day-to-day existence in the gritty, cyberpunk-meets-fantasy world of the Sixth World. Lifestyles are a key mechanic in the game, affecting not just flavor but also gameplay elements like healing, contacts, and how much attention a character might draw from others (e.g., corps, gangs, or authorities).
"#;

pub const NUYEN: &str = r#"
The Nuyen (pronounced New Yen), symbol ¥, is the currency of Japan and the primary monetary unit of international trade. It replaced the older Yen as Japan's currency on June 1 2012 as part of the Yamato act.
"#;

// endregion:  --- Resources

// region:  --- Attributes
pub const BODY: &str = r#"
Measures physical durability and toughness. Determines how much damage you can take (hit points, essentially) and resist toxins, diseases, or exhaustion. High BODY suits tanks or anyone expecting to get shot at.
"#;

pub const AGILITY: &str = r#"
Governs dexterity, coordination, and finesse. Key for shooting guns, sneaking, picking locks, or dodging. A must for street samurai, infiltrators, or anyone relying on precise movement.
"#;

pub const REACTION: &str = r#"
Reflects reflexes and quickness. Affects initiative (how fast you act in combat) and dodging attacks. Pairs with INTUITION for surprise checks. Great for drivers, gunners, or anyone needing split-second timing.
"#;

pub const STRENGTH: &str = r#"
Raw physical power. Controls melee damage, lifting capacity, and climbing or jumping ability. Essential for close-combat brawlers or anyone swinging a katana or fist.
"#;

pub const WILLPOWER: &str = r#"
Mental resilience and determination. Resists magic (like spell damage), fear, and mental strain. Also fuels spellcasting stamina for mages. High WILLPOWER keeps you sane in the shadows.
"#;

pub const LOGIC: &str = r#"
Intelligence and analytical ability. Drives hacking, technical skills (like fixing drones), and knowledge checks. Deckers, riggers, and brainy types thrive with high LOGIC.
"#;

pub const INTUITION: &str = r#"
Gut instinct and perception. Covers noticing details, avoiding ambushes (with REACTION), and street smarts. Useful for shamans, investigators, or anyone navigating the sprawl’s chaos.
"#;

pub const CHARISMA: &str = r#"
Charm, social savvy, and presence. Powers negotiation, intimidation, and conning. Critical for faces (social specialists) and some mages (spirit summoning). Elves love this one.
"#;

pub const EDGE: &str = r#"
Luck and grit. A meta-attribute letting you reroll dice, cheat death, or seize the moment. It’s your “get out of jail free” card—rare and precious. Higher EDGE means more clutch plays.
"#;

pub const MAGIC: &str = r#"
Mystical power for awakened characters (mages, shamans, adepts). Determines spellcasting strength, spirit control, or physical adept abilities. Zero for mundanes; high for magical heavyweights.
"#;

pub const RESONANCE: &str = r#"
Technomancer mojo. The Matrix equivalent of MAGIC, it fuels living code manipulation, sprite summoning, and resisting dumpshock. Only technomancers have it; mundanes and mages don’t.
"#;

// endregion:  --- Attributes

// region:  --- Derived Attributes

pub const INITIATIVE: &str = r#"
Determines turn order in combat. Calculated as REACTION + INTUITION + 1d6 (or more dice with boosts like Wired Reflexes). Higher totals act first; each round, you subtract 10 and go again until it’s zero or negative. Augments, spells, or drugs can juice this up.
"#;

pub const LIMIT_PHYSICAL: &str = r#"
Caps on how many successes (hits) you can keep from a dice roll, reflecting natural boundaries.
Physical (PHY): Based on (STR × 2 + BOD + REA) ÷ 3. Limits tests like athletics or combat.
"#;

pub const LIMIT_MENTAL: &str = r#"
Caps on how many successes (hits) you can keep from a dice roll, reflecting natural boundaries.
Mental (MEN): Based on (LOG × 2 + INT + WIL) ÷ 3. Limits hacking, perception, or memory.
"#;

pub const LIMIT_SOCIAL: &str = r#"
Caps on how many successes (hits) you can keep from a dice roll, reflecting natural boundaries.
Social (SOC): Based on (CHA × 2 + WIL + ESS) ÷ 3. Limits conning or negotiation.
"#;

pub const MONITOR_PHYSICAL: &str = r#"
Tracks damage a character can take before dropping.
Physical Condition Monitor (PHY): Hit points, essentially. Calculated as (BODY ÷ 2, rounded up) + 8. Boxes fill with physical damage (bullets, fists); overflow means you’re out or dying.
"#;

pub const MONITOR_STUN: &str = r#"
Tracks damage a character can take before dropping.
Stun Condition Monitor (SOC): Mental/physical fatigue. Calculated as (WILLPOWER ÷ 2, rounded up) + 8. Fills with stun damage (tasers, spells); full means unconsciousness.
"#;

pub const ESSENCE: &str = r#"
A measure of your “humanity” or soul, starting at 6 for most metahumans. Cyberware and bioware implants reduce it—lose too much (to 0 or below), and you die or become a cyberzombie. Affects MAGIC/RESONANCE (lower Essence weakens them) and Social Limit. It’s the cost of chrome.
"#;

pub const EDGE_POINTS: &str = r#"
Your pool of luck, tied to the EDGE attribute. Spend a point to reroll failures, boost a roll, act first, or escape death. Refreshes each session or run (GM’s call). In 6th Edition, Edge is more dynamic, gained/spent fluidly in combat, capped by your EDGE stat.
"#;

pub const ARMOR: &str = r#"
Protection against damage. Rated numerically (e.g., Armor Jacket is 12). Added to BODY when soaking damage—roll that total, and each hit reduces incoming damage. Can be penetrated by AP (Armor Penetration) from weapons. Stacks with cyberware or magic, but encumbrance might slow you down if too heavy.
"#;

// ENDREGION:  --- DERIVED ATTRIBUTES
