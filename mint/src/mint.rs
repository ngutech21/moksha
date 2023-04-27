use cashurs_core::model::MintKeyset;

use crate::lightning::Lightning;

#[derive(Clone)]
pub struct Mint {
    pub lightning: Lightning,
    pub keyset: MintKeyset,
}

impl Mint {
    pub fn new(secret: String, lightning: Lightning) -> Self {
        Self {
            lightning,
            keyset: MintKeyset::new(secret),
        }
    }
}
