use cashurs_core::model::Proofs;

pub struct Database {}

impl Database {
    pub fn read_proofs(&self) -> Proofs {
        vec![]
    }

    pub fn write_proofs(&self, proofs: Proofs) {}
}
