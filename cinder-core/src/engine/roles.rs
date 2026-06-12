#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RoleHandler {
    Dispatch,
    CommandParser,
    StateReader,
    Aggregation,
    Planner,
    MenuIntentClarifier,
    DialogueGrounder,
    ActorDialogue,
    Reducer,
    Narrator,
}

impl RoleHandler {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "dispatch" => Some(Self::Dispatch),
            "command_parser" => Some(Self::CommandParser),
            "state_reader" => Some(Self::StateReader),
            "aggregation" => Some(Self::Aggregation),
            "planner" => Some(Self::Planner),
            "menu_intent_clarifier" => Some(Self::MenuIntentClarifier),
            "dialogue_grounder" => Some(Self::DialogueGrounder),
            "actor_dialogue" => Some(Self::ActorDialogue),
            "reducer" => Some(Self::Reducer),
            "narrator" => Some(Self::Narrator),
            _ => None,
        }
    }
}
