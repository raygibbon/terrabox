use crate::WindowId;
use terrabox::KeyEvent;

/// An application-owned command identity, shared by menus and shortcuts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Global,
    Window(WindowId),
}

/// Replaceable single-key bindings. Local scope wins over global scope.
#[derive(Default)]
pub struct Bindings(Vec<(Scope, KeyEvent, CommandId)>);

impl Bindings {
    pub fn bind(&mut self, scope: Scope, key: KeyEvent, command: CommandId) {
        self.unbind(scope, key);
        self.0.push((scope, key, command));
    }

    pub fn unbind(&mut self, scope: Scope, key: KeyEvent) {
        self.0
            .retain(|(existing, bound, _)| *existing != scope || *bound != key);
    }

    pub fn resolve(&self, scope: Scope, key: KeyEvent) -> Option<CommandId> {
        self.0
            .iter()
            .find(|(existing, bound, _)| *existing == scope && *bound == key)
            .map(|(_, _, command)| *command)
    }
}
