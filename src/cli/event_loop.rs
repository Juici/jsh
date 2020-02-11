use anyhow::Result;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;

use crate::cli::app::Return;
use crate::cli::tty::Event;

const INPUT_CHANNEL_SIZE: usize = 128;

pub struct EventLoopTx {
    input_tx: Sender<Event>,
    return_tx: Sender<Result<Return>>,
}

impl Clone for EventLoopTx {
    fn clone(&self) -> Self {
        let input_tx = self.input_tx.clone();
        let return_tx = self.return_tx.clone();

        EventLoopTx {
            input_tx,
            return_tx,
        }
    }
}

impl EventLoopTx {
    pub async fn send_input(&mut self, event: Event) -> Result<()> {
        Ok(self.input_tx.send(event).await?)
    }

    pub async fn send_return(&mut self, ret: Result<Return>) -> Result<()> {
        Ok(self.return_tx.send(ret).await?)
    }
}

pub struct EventLoop {
    input_rx: Receiver<Event>,
    return_rx: Receiver<Result<Return>>,

    handle_tx: Option<Sender<Event>>,
    redraw_tx: Option<Sender<RedrawFlag>>,

    inner_redraw_tx: Mutex<Sender<InnerRedrawFlag>>,
    inner_redraw_rx: Receiver<InnerRedrawFlag>,
}

impl EventLoop {
    pub fn new() -> (EventLoop, EventLoopTx) {
        let (input_tx, input_rx) = mpsc::channel(INPUT_CHANNEL_SIZE);
        let (return_tx, return_rx) = mpsc::channel(1);

        let (inner_redraw_tx, inner_redraw_rx) = mpsc::channel(1);

        let event_loop = EventLoop {
            input_rx,
            return_rx,

            handle_tx: None,
            redraw_tx: None,

            inner_redraw_tx: Mutex::new(inner_redraw_tx),
            inner_redraw_rx,
        };

        let event_loop_tx = EventLoopTx {
            input_tx,
            return_tx,
        };

        (event_loop, event_loop_tx)
    }

    pub fn set_handle_tx(&mut self, handle_tx: Sender<Event>) {
        self.handle_tx = Some(handle_tx);
    }

    pub fn set_redraw_tx(&mut self, redraw_tx: Sender<RedrawFlag>) {
        self.redraw_tx = Some(redraw_tx);
    }

    pub async fn redraw(&self, full: bool) -> Result<()> {
        let mut tx = self.inner_redraw_tx.lock().await;

        let flag = if full {
            InnerRedrawFlag::Full
        } else {
            InnerRedrawFlag::Redraw
        };

        tx.send(flag).await?;

        Ok(())
    }

    pub async fn run(&mut self) -> Result<Return> {
        loop {
            tokio::select! {
                Some(event) = self.input_rx.recv() => {
                    self.handle_event(event).await?;

                    'consume_events: loop {
                        if let Ok(ret) = self.return_rx.try_recv() {
                            self.handle_redraw(RedrawFlag::Final).await?;
                            return ret;
                        }

                        match self.input_rx.try_recv() {
                            Ok(event) => self.handle_event(event).await?,
                            Err(_) => break 'consume_events,
                        }
                    }
                }
                Some(ret) = self.return_rx.recv() => {
                    self.handle_redraw(RedrawFlag::Final).await?;
                    return ret;
                }
            }
        }
    }

    async fn handle_event(&self, event: Event) -> Result<()> {
        println!("recv event: {:?}", event); // TODO
        Ok(())
    }

    async fn handle_redraw(&self, redraw: RedrawFlag) -> Result<()> {
        println!("redraw: {:?}", redraw); // TODO
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RedrawFlag {
    Full,
    Final,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum InnerRedrawFlag {
    Redraw,
    Full,
}
