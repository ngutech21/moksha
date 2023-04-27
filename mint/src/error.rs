use thiserror::Error;

#[derive(Error, Debug)]
pub enum CashuMintError {
    #[error("LnbitsError {0}")]
    Lnbits(#[from] lnbits_rust::LNBitsError),

    #[error("{0}")]
    Db(String),
}
