use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TermError {
    #[error("failed to get terminal attributes")]
    GetAttributes(#[source] io::Error),
    #[error("failed to set terminal attributes")]
    SetAttributes(#[source] io::Error),
    #[error("failed to setup VT")]
    SetupVt(#[source] anyhow::Error),
    #[error("failed to read event")]
    ReadEvent(#[source] io::Error),
    #[error("failed to get terminal size")]
    GetSize(#[source] io::Error),
    #[error("failed to flush unread input")]
    FlushInput(#[source] io::Error),
}
