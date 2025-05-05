use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use color_eyre::eyre::Result;

use futures::{FutureExt, StreamExt};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{Event as CrosstermEvent, KeyEvent, KeyEventKind, MouseEvent},
};
use ratatui_image::picker::Picker;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::{MIN_HEIGHT, MIN_WIDTH};

#[derive(Clone, Debug)]
pub enum TuiEvent {
    Init,
    // Quit,
    Error,
    // Closed,
    Tick,
    Render,
    FocusGained,
    FocusLost,
    // TODO: Maybe I don't need copypasta anymore?
    Paste(String),
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

pub struct Tui {
    pub terminal: DefaultTerminal,
    pub picker: Picker,
    pub task: JoinHandle<()>,
    pub cancellation_token: CancellationToken,
    pub event_rx: UnboundedReceiver<TuiEvent>,
    pub event_tx: UnboundedSender<TuiEvent>,
    pub frame_rate: f64,
    pub tick_rate: f64,
    // pub mouse: bool,
    // pub paste: bool,
}

impl Tui {
    pub fn new() -> Result<Self> {
        let tick_rate = 4.0;
        let frame_rate = 60.0;
        let terminal = ratatui::init();
        let picker = Picker::from_query_stdio().unwrap_or(Picker::from_fontsize((18, 42)));
        log::debug!("Picker has been set to: {picker:#?}");
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let cancellation_token = CancellationToken::new();
        let task = tokio::spawn(async {});
        // let mouse = false;
        // let paste = true;
        Ok(Self {
            terminal,
            picker,
            task,
            cancellation_token,
            event_rx,
            event_tx,
            frame_rate,
            tick_rate,
            // mouse,
            // paste,
        })
    }

    pub fn tick_rate(mut self, tick_rate: f64) -> Self {
        self.tick_rate = tick_rate;
        self
    }

    pub fn frame_rate(mut self, frame_rate: f64) -> Self {
        self.frame_rate = frame_rate;
        self
    }

    // pub fn mouse(mut self, mouse: bool) -> Self {
    //     self.mouse = mouse;
    //     self
    // }

    // pub fn paste(mut self, paste: bool) -> Self {
    //     self.paste = paste;
    //     self
    // }

    pub fn start(&mut self) {
        let tick_delay = std::time::Duration::from_secs_f64(1.0 / self.tick_rate);
        let render_delay = std::time::Duration::from_secs_f64(1.0 / self.frame_rate);
        self.cancel();
        self.cancellation_token = CancellationToken::new();
        let _cancellation_token = self.cancellation_token.clone();
        let _event_tx = self.event_tx.clone();
        self.task = tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_delay);
            let mut render_interval = tokio::time::interval(render_delay);
            _event_tx.send(TuiEvent::Init).unwrap();
            loop {
                let tick_delay = tick_interval.tick();
                let render_delay = render_interval.tick();
                let crossterm_event = reader.next().fuse();
                tokio::select! {
                  _ = _cancellation_token.cancelled() => {
                    break;
                  }
                  maybe_event = crossterm_event => {
                    match maybe_event {
                      Some(Ok(evt)) => {
                        match evt {
                          CrosstermEvent::Key(key) => {
                            if key.kind == KeyEventKind::Press {
                              _event_tx.send(TuiEvent::Key(key)).unwrap();
                            }
                          },
                          CrosstermEvent::Mouse(mouse) => {
                            _event_tx.send(TuiEvent::Mouse(mouse)).unwrap();
                          },
                          CrosstermEvent::Resize(x, y) => {
                            _event_tx.send(TuiEvent::Resize(x, y)).unwrap();
                          },
                          CrosstermEvent::FocusLost => {
                            _event_tx.send(TuiEvent::FocusLost).unwrap();
                          },
                          CrosstermEvent::FocusGained => {
                            _event_tx.send(TuiEvent::FocusGained).unwrap();
                          },
                          CrosstermEvent::Paste(s) => {
                            _event_tx.send(TuiEvent::Paste(s)).unwrap();
                          },
                        }
                      }
                      Some(Err(_)) => {
                        _event_tx.send(TuiEvent::Error).unwrap();
                      }
                      None => {},
                    }
                  },
                  _ = tick_delay => {
                      _event_tx.send(TuiEvent::Tick).unwrap();
                  },
                  _ = render_delay => {
                      _event_tx.send(TuiEvent::Render).unwrap();
                  },
                }
            }
        });
    }

    pub fn stop(&self) -> Result<()> {
        self.cancel();
        let mut counter = 0;
        while !self.task.is_finished() {
            std::thread::sleep(Duration::from_millis(1));
            counter += 1;
            if counter > 50 {
                self.task.abort();
            }
            if counter > 100 {
                log::error!("Failed to abort task in 100 milliseconds for unknown reason");
                break;
            }
        }
        Ok(())
    }

    pub fn enter(&mut self) -> Result<()> {
        self.ensure_minimum_terminal_size()?;
        self.start();
        Ok(())
    }

    pub fn exit(&mut self) -> Result<()> {
        log::info!("Sharad exit: {}", chrono::Local::now());
        self.stop()?;
        ratatui::restore();
        Ok(())
    }

    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    // pub fn suspend(&mut self) -> Result<()> {
    //     self.exit()?;
    //     #[cfg(not(windows))]
    //     signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP)?;
    //     Ok(())
    // }

    // pub fn resume(&mut self) -> Result<()> {
    //     self.enter()?;
    //     Ok(())
    // }

    pub async fn next(&mut self) -> Option<TuiEvent> {
        self.event_rx.recv().await
    }

    pub fn ensure_minimum_terminal_size(&self) -> Result<()> {
        let size = self.terminal.size()?; // Get current size of the terminal.
        // If the current size is less than minimum, resize to the minimum required.
        if size.width < MIN_WIDTH || size.height < MIN_HEIGHT {
            //need to send a TuiEvent::Resize(size)
            self.event_tx
                .send(TuiEvent::Resize(MIN_WIDTH, MIN_HEIGHT))?;
        }
        Ok(())
    }
}

impl Deref for Tui {
    type Target = DefaultTerminal;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for Tui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        self.exit().unwrap();
    }
}
