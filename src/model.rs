use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindedMessage {
    pub amount: u64,
    pub b_: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindedSignature {
    pub amount: u64,
    pub c_: String,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    pub amount: u64,
    pub secret: String,
    pub c: String,
    pub id: Option<String>,
    pub script: Option<P2SHScript>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2SHScript {}

pub type Proofs = Vec<Proof>;
