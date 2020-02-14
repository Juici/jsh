use async_trait::async_trait;

use crate::cli::term::buffer::Buffer;
use crate::cli::tty::Event;

pub trait Widget: Render + Handle {}

#[async_trait]
pub trait Render {
    async fn render(&mut self, width: u16, height: u16) -> Buffer;
}

#[async_trait]
pub trait Handle {
    async fn handle(&mut self, event: Event) -> bool;
}
