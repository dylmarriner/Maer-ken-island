//! Explicit phase-10/11 runtime surface.
//!
//! `mk_engine` owns the world simulation pipeline at the crate root. Later-stage
//! agent and human runtime layers remain available, but only through this
//! dedicated boundary so they do not read as canonical top-level world exports.

pub mod agent {
    pub use crate::agents::{
        ActionKind, AffectSnapshot, Agent, AgentAction, AgentId, AgentStepAudit, AgentSystem,
        AgentWorldObservation, ArousalBand, CarryingState, CognitiveSnapshot, DecisionAudit,
        DecisionEngine, EmotionalValence, EndocrineSnapshot, GridPosition, HormoneAxis, ItemKind,
        ItemStack, MemorySystem, MortalityReason, MortalitySnapshot, NervousSystemSnapshot,
        PhysiologySnapshot, VitalStatus, WillSnapshot, WillStatus,
    };
}

pub mod human {
    pub use crate::humans::computer_bridge::{ComputerBridgeHandle, HttpComputerBridge};
    pub use crate::humans::thought::generate_thought;
    pub use crate::humans::{
        BiologicalSex, CognitiveMode, ConversationEvent, ConversationRelationship, CultureSnapshot,
        DevelopmentSnapshot, DevelopmentStage, DialogueLine, GeneticsSnapshot, HumanBeing,
        HumanCognitionSnapshot, HumanFactory, HumanRegistry, HumanStatus, HumanSystem,
        LanguageMode, LanguageSnapshot, SocialSystemsSnapshot, TechnologySnapshot,
    };
}

pub use agent::{
    ActionKind, AffectSnapshot, Agent, AgentAction, AgentId, AgentStepAudit, AgentSystem,
    AgentWorldObservation, ArousalBand, CarryingState, CognitiveSnapshot, DecisionAudit,
    DecisionEngine, EmotionalValence, EndocrineSnapshot, GridPosition, HormoneAxis, ItemKind,
    ItemStack, MemorySystem, MortalityReason, MortalitySnapshot, NervousSystemSnapshot,
    PhysiologySnapshot, VitalStatus, WillSnapshot, WillStatus,
};
pub use human::{
    generate_thought, BiologicalSex, CognitiveMode, ComputerBridgeHandle, ConversationEvent,
    ConversationRelationship, CultureSnapshot, DevelopmentSnapshot, DevelopmentStage, DialogueLine,
    GeneticsSnapshot, HttpComputerBridge, HumanBeing, HumanCognitionSnapshot, HumanFactory,
    HumanRegistry, HumanStatus, HumanSystem, LanguageMode, LanguageSnapshot, SocialSystemsSnapshot,
    TechnologySnapshot,
};
