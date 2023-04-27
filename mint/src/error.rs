use thiserror::Error;

#[derive(Error, Debug)]
pub enum CashuMintError {
    #[error("LnbitsError {0}")]
    Lnbits(#[from] lnbits_rust::LNBitsError),
}
