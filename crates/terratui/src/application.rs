use crate::{CommandId, Desktop};
use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::time::Instant;
use terrabox::{Deadline, Event, Terminal};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimerId(pub u64);

/// Framework commands, raw unconsumed input, one-shot timers and user messages.
pub enum AppEvent<Message> {
    Terminal(Event),
    Command(CommandId),
    Timer(TimerId),
    User(Message),
}

/// Application callbacks run on the UI thread. Domain state stays in this object.
pub trait App {
    type Message;
    fn init(&mut self, _context: &mut Context<'_, Self::Message>) -> io::Result<()> {
        Ok(())
    }
    fn event(
        &mut self,
        event: AppEvent<Self::Message>,
        context: &mut Context<'_, Self::Message>,
    ) -> io::Result<()>;
    fn shutdown(&mut self, _context: &mut Context<'_, Self::Message>) -> io::Result<()> {
        Ok(())
    }
}

/// Mutations should request redraw when they affect visible application state.
pub struct Context<'a, Message> {
    pub desktop: &'a mut Desktop,
    terminal: &'a mut Terminal,
    messages: &'a mut VecDeque<Message>,
    timers: &'a mut BTreeMap<TimerId, Deadline>,
    redraw: &'a mut bool,
    running: &'a mut bool,
}

impl<Message> Context<'_, Message> {
    pub fn redraw(&mut self) {
        *self.redraw = true;
    }
    pub fn quit(&mut self) {
        *self.running = false;
    }
    pub fn post(&mut self, message: Message) {
        self.messages.push_back(message);
    }
    /// Replaces an existing timer of the same identity. Timers fire once.
    pub fn set_timer(&mut self, id: TimerId, deadline: Deadline) {
        self.timers.insert(id, deadline);
    }
    pub fn cancel_timer(&mut self, id: TimerId) {
        self.timers.remove(&id);
    }
    pub fn size(&self) -> terrabox::Size {
        self.terminal.size()
    }
    pub fn set_mouse_enabled(&mut self, enabled: bool) -> io::Result<()> {
        self.terminal.set_mouse_enabled(enabled)
    }
    pub fn set_input_mode(&mut self, mode: terrabox::InputMode) {
        self.terminal.set_input_mode(mode);
    }

    /// Suspend for a child process and restore the terminal, propagating failures.
    pub fn run_command(
        &mut self,
        command: &mut std::process::Command,
    ) -> io::Result<std::process::ExitStatus> {
        *self.redraw = true;
        self.terminal.run_command(command)
    }
}

/// Owns the terminal for a blocking, deadline-driven application lifecycle.
pub struct Application<Model: App> {
    pub desktop: Desktop,
    model: Model,
    messages: VecDeque<Model::Message>,
    timers: BTreeMap<TimerId, Deadline>,
    redraw: bool,
    running: bool,
}

impl<Model: App> Application<Model> {
    pub fn new(model: Model, desktop: Desktop) -> Self {
        Self {
            desktop,
            model,
            messages: VecDeque::new(),
            timers: BTreeMap::new(),
            redraw: true,
            running: true,
        }
    }

    pub fn run(self) -> io::Result<()> {
        self.run_on(Terminal::new()?)
    }

    /// Use an explicitly connected terminal. Ownership ensures cleanup on return.
    pub fn run_on(mut self, mut terminal: Terminal) -> io::Result<()> {
        let result = self
            .with_context(&mut terminal, |model, context| model.init(context))
            .and_then(|()| self.event_loop(&mut terminal));
        let shutdown = self.with_context(&mut terminal, |model, context| model.shutdown(context));
        result.and(shutdown)
    }

    fn with_context(
        &mut self,
        terminal: &mut Terminal,
        callback: impl FnOnce(&mut Model, &mut Context<'_, Model::Message>) -> io::Result<()>,
    ) -> io::Result<()> {
        let mut context = Context {
            desktop: &mut self.desktop,
            terminal,
            messages: &mut self.messages,
            timers: &mut self.timers,
            redraw: &mut self.redraw,
            running: &mut self.running,
        };
        callback(&mut self.model, &mut context)
    }

    fn dispatch(
        &mut self,
        terminal: &mut Terminal,
        event: AppEvent<Model::Message>,
    ) -> io::Result<()> {
        self.with_context(terminal, |model, context| model.event(event, context))
    }

    fn event_loop(&mut self, terminal: &mut Terminal) -> io::Result<()> {
        while self.running {
            let now = Instant::now();
            let due = self
                .timers
                .iter()
                .filter(|(_, deadline)| deadline.is_due(now))
                .map(|(id, _)| *id)
                .collect::<Vec<_>>();
            for id in due {
                if self
                    .timers
                    .get(&id)
                    .is_some_and(|deadline| deadline.is_due(now))
                {
                    self.timers.remove(&id);
                    self.dispatch(terminal, AppEvent::Timer(id))?;
                    if !self.running {
                        break;
                    }
                }
            }
            if !self.running {
                break;
            }
            if let Some(message) = self.messages.pop_front() {
                self.dispatch(terminal, AppEvent::User(message))?;
            }
            if !self.running {
                break;
            }
            if self.redraw {
                let surface = self.desktop.render(terminal.size());
                terminal.draw_surface(&surface, terrabox::Position::default());
                terminal.hide_cursor();
                terminal.present()?;
                self.redraw = false;
            }
            if !self.messages.is_empty() {
                continue;
            }
            let next = self.timers.values().min().copied();
            let event = if let Some(deadline) = next {
                terminal.read_event_timeout(deadline.remaining(Instant::now()))?
            } else {
                Some(terminal.read_event()?)
            };
            let Some(event) = event else {
                continue;
            };
            if matches!(event, Event::Resize(_)) {
                self.redraw = true;
            }
            let result = self.desktop.route(&event);
            self.redraw |= result.redraw;
            if let Some(command) = result.command {
                self.dispatch(terminal, AppEvent::Command(command))?;
            } else if !result.consumed || matches!(event, Event::Resize(_)) {
                self.dispatch(terminal, AppEvent::Terminal(event))?;
            }
        }
        Ok(())
    }
}
