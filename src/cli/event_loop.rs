use anyhow::Result;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;

use crate::cli::app::Return;
use crate::cli::tty::Event;

const INPUT_CHANNEL_SIZE: usize = 128;

pub struct EventLoopTx {
    input_tx: Sender<Event>,
    return_tx: Sender<Result<Return>>,
    redraw_tx: Sender<Redraw>,
}

impl Clone for EventLoopTx {
    fn clone(&self) -> Self {
        let input_tx = self.input_tx.clone();
        let return_tx = self.return_tx.clone();
        let redraw_tx = self.redraw_tx.clone();

        EventLoopTx {
            input_tx,
            return_tx,
            redraw_tx,
        }
    }
}

impl EventLoopTx {
    pub async fn send_input(&mut self, event: Event) -> Result<()> {
        Ok(self.input_tx.send(event).await?)
    }

    #[inline]
    async fn inner_send_return(
        tx: &mut Sender<Result<Return>>,
        ret: Result<Return>,
    ) -> Result<bool> {
        match tx.try_send(ret) {
            Ok(()) => Ok(true),
            Err(TrySendError::Full(_)) => Ok(false), // TODO: Maybe return an `Err(Return)` value.
            Err(err @ TrySendError::Closed(_)) => Err(anyhow::anyhow!(err)),
        }
    }

    pub async fn send_return(&mut self, ret: Result<Return>) -> Result<bool> {
        Self::inner_send_return(&mut self.return_tx, ret).await
    }

    pub async fn send_return_clone(&self, ret: Result<Return>) -> Result<bool> {
        Self::inner_send_return(&mut self.return_tx.clone(), ret).await
    }

    #[inline]
    async fn inner_send_redraw(
        tx: &mut Sender<Redraw>,
        full: bool,
        size: Option<(u16, u16)>,
    ) -> Result<()> {
        let flags = if full {
            RedrawFlags::FULL
        } else {
            RedrawFlags::empty()
        };

        Ok(tx.send(Redraw { size, flags }).await?)
    }

    pub async fn send_redraw(&mut self, full: bool, size: Option<(u16, u16)>) -> Result<()> {
        Self::inner_send_redraw(&mut self.redraw_tx, full, size).await
    }

    pub async fn send_redraw_clone(&self, full: bool, size: Option<(u16, u16)>) -> Result<()> {
        Self::inner_send_redraw(&mut self.redraw_tx.clone(), full, size).await
    }
}

pub struct EventLoop {
    input_rx: Receiver<Event>,
    return_rx: Receiver<Result<Return>>,

    event_tx: Option<Sender<Event>>,
    redraw_tx: Option<Sender<Redraw>>,

    inner_redraw_tx: Mutex<Sender<Redraw>>,
    inner_redraw_rx: Receiver<Redraw>,
}

impl EventLoop {
    pub fn new() -> (EventLoop, EventLoopTx) {
        let (input_tx, input_rx) = mpsc::channel(INPUT_CHANNEL_SIZE);
        let (return_tx, return_rx) = mpsc::channel(1);

        let (inner_redraw_tx, inner_redraw_rx) = mpsc::channel(1);

        let event_loop = EventLoop {
            input_rx,
            return_rx,

            event_tx: None,
            redraw_tx: None,

            inner_redraw_tx: Mutex::new(inner_redraw_tx.clone()),
            inner_redraw_rx,
        };

        let event_loop_tx = EventLoopTx {
            input_tx,
            return_tx,
            redraw_tx: inner_redraw_tx,
        };

        (event_loop, event_loop_tx)
    }

    pub fn set_event_tx(&mut self, event_tx: Sender<Event>) {
        self.event_tx = Some(event_tx);
    }

    pub fn set_redraw_tx(&mut self, redraw_tx: Sender<Redraw>) {
        self.redraw_tx = Some(redraw_tx);
    }

    pub async fn run(&mut self) -> Result<Return> {
        let mut redraw = Redraw {
            size: None,
            flags: RedrawFlags::empty(),
        };

        loop {
            // Send redraw.
            if let Some(redraw_tx) = self.redraw_tx.as_mut() {
                redraw_tx.send(redraw).await?;
            }
            redraw.flags = RedrawFlags::empty();

            tokio::select! {
                // Received an event.
                Some(event) = self.input_rx.recv() => {
                    self.handle_event(event).await?;

                    // Keep consuming available events to minimize redraws.
                    'consume_events: loop {
                        if let Ok(ret) = self.return_rx.try_recv() {
                            self.handle_redraw(RedrawFlags::FINAL).await?;
                            return ret;
                        }

                        match self.input_rx.try_recv() {
                            Ok(event) => self.handle_event(event).await?,
                            Err(_) => break 'consume_events,
                        }
                    }
                }
                // Received return message.
                Some(ret) = self.return_rx.recv() => {
                    self.handle_redraw(RedrawFlags::FINAL).await?;
                    return ret;
                }
                // Received a redraw request.
                Some(Redraw { size, flags }) = self.inner_redraw_rx.recv() => {
                    // Update size if sent.
                    if let Some(size) = size {
                        redraw.size = Some(size);
                    }

                    redraw.flags = flags;
                }
            }
        }
    }

    async fn handle_event(&mut self, event: Event) -> Result<()> {
        if let Some(event_tx) = self.event_tx.as_mut() {
            event_tx.send(event).await?;
        }
        Ok(())
    }

    async fn handle_redraw(&mut self, flags: RedrawFlags) -> Result<()> {
        if let Some(redraw_tx) = self.redraw_tx.as_mut() {
            redraw_tx.send(Redraw { size: None, flags }).await?;
        }
        Ok(())
    }
}

bitflags::bitflags! {
    pub struct RedrawFlags: u8 {
        const FULL = 0b01;
        const FINAL = 0b10;
    }
}

impl RedrawFlags {
    pub fn is_final(self) -> bool {
        self.contains(RedrawFlags::FINAL)
    }

    pub fn is_full(self) -> bool {
        self.contains(RedrawFlags::FULL)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Redraw {
    pub size: Option<(u16, u16)>,
    pub flags: RedrawFlags,
}
