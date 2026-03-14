mod trait_decls;
mod pool;
mod executor;

pub use trait_decls::*;
pub use executor::*;
pub use pool::*;

pub struct BranchAgent {
    info: AgentInfo,
}

impl BranchAgent {
    pub fn new(info: AgentInfo) -> Self {
        Self { info }
    }
}

pub struct LeafAgent {
    info: AgentInfo,
}

impl LeafAgent {
    pub fn new(info: AgentInfo) -> Self {
        Self { info }
    }
}
