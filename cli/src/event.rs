use color_eyre::Result;
use ratatui::crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug)]
pub enum Event {
    Tick,
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

#[derive(Debug)]
pub struct EventHandler {
    #[allow(dead_code)]
    sender: mpsc::Sender<Event>,
    receiver: mpsc::Receiver<Event>,

    #[allow(dead_code)]
    handler: thread::JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate: u64) -> Self {
        let tick_rate = Duration::from_millis(tick_rate);
        let (sender, receiver) = mpsc::channel();
        let handler = {
            let sender = sender.clone();
            thread::spawn(move || {
                let mut last_tick = Instant::now();
                loop {
                    let timeout = tick_rate
                        .checked_sub(last_tick.elapsed())
                        .unwrap_or(tick_rate);

                    match event::poll(timeout) {
                        Ok(true) => {
                            let Ok(crossterm_event) = event::read() else {
                                break;
                            };

                            let result = match crossterm_event {
                                CrosstermEvent::Key(e) => {
                                    if matches!(
                                        e.kind,
                                        event::KeyEventKind::Press | event::KeyEventKind::Repeat
                                    ) {
                                        sender.send(Event::Key(e))
                                    } else {
                                        Ok(())
                                    }
                                }
                                CrosstermEvent::Mouse(e) => sender.send(Event::Mouse(e)),
                                CrosstermEvent::Resize(w, h) => sender.send(Event::Resize(w, h)),
                                _ => Ok(()),
                            };

                            if result.is_err() {
                                break;
                            }
                        }
                        Ok(false) => {}
                        Err(_) => break,
                    }

                    if last_tick.elapsed() >= tick_rate {
                        if sender.send(Event::Tick).is_err() {
                            break;
                        }
                        last_tick = Instant::now();
                    }
                }
            })
        };
        Self {
            sender,
            receiver,
            handler,
        }
    }

    pub fn next(&self) -> Result<Event> {
        Ok(self.receiver.recv()?)
    }
}
