use thiserror::Error;

#[derive(Error, Debug)]
pub enum CashuCoreError {
    #[error("Secp256k1Error {0}")]
    Secp256k1Error(#[from] secp256k1::Error),
}
