use serde::{Deserialize, Serialize};

/// A single entry in an actor's `drops` table.
///
/// The map key is normally the scatters-on-defeat item id; for `Weighted`
/// pools the key is just a synthetic label (e.g. `"pawn-kit"`) since the
/// actual items live in the pool entries. Supported spec shapes:
/// - a plain count: the item always scatters on defeat;
/// - a conditional spec whose story-var gate lets packs withhold loot based on
///   how the run went (e.g. a boss ring only from a clean pacifist floor);
/// - a chance spec that rolls a percentage per defeat;
/// - a weighted pool that rolls entries by weight.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DropSpec {
    Always(u32),
    /// Declared before `Conditional` so a `{"count": n, "chance_percent": p}`
    /// spec is not mistaken for a conditional that merely defaults its gate.
    Chance(DropChanceSpec),
    Conditional(DropConditionSpec),
    Weighted(DropPoolSpec),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropConditionSpec {
    pub count: u32,
    /// When truthy, the item is not dropped and is excluded from the defeat
    /// drop narration.
    #[serde(default)]
    pub skip_when_story_var: String,
}

/// An item that only scatters on a percentage of defeats. `chance_percent`
/// must be `0..=100`; the roll is made once per defeat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropChanceSpec {
    #[serde(default = "one_drop_count")]
    pub count: u32,
    pub chance_percent: u32,
}

/// A weighted drop pool: each defeat rolls `rolls` times and picks an entry by
/// weight, so packs can build "the pawn scatters one piece of its kit" tables.
/// The pool's `drops` map key is a synthetic label, not an item id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DropPoolSpec {
    #[serde(default = "one_drop_count")]
    pub rolls: u32,
    pub entries: Vec<DropPoolEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DropPoolEntry {
    pub item_id: String,
    /// Relative likelihood this entry is picked for each roll.
    pub weight: u32,
    pub count: u32,
}

impl Default for DropPoolSpec {
    fn default() -> Self {
        Self {
            rolls: 1,
            entries: Vec::new(),
        }
    }
}

impl Default for DropPoolEntry {
    fn default() -> Self {
        Self {
            item_id: String::new(),
            weight: 1,
            count: 1,
        }
    }
}

fn one_drop_count() -> u32 {
    1
}