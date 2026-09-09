use std::io;

use argon2::password_hash;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Input(#[from] io::Error),
    #[error("{0}")]
    Hashing(password_hash::Error),
}
