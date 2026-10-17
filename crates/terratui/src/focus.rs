/// Stable window identity, independent of document or content identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WindowId(pub u64);

/// Ordered base focus and a stack of modal captures with restoration.
#[derive(Default)]
pub struct Focus {
    order: Vec<WindowId>,
    active: Option<WindowId>,
    modals: Vec<(WindowId, Option<WindowId>)>,
}

impl Focus {
    pub fn active(&self) -> Option<WindowId> {
        self.active
    }

    pub fn modal(&self) -> Option<WindowId> {
        self.modals.last().map(|(id, _)| *id)
    }

    pub fn set_order(&mut self, order: Vec<WindowId>) {
        self.order = order;
        if self.modal().is_none() && !self.active.is_some_and(|id| self.order.contains(&id)) {
            self.active = self.order.first().copied();
        }
    }

    pub fn select(&mut self, id: WindowId) -> bool {
        if self.modal().is_some() || !self.order.contains(&id) {
            return false;
        }
        self.active = Some(id);
        true
    }

    pub fn advance(&mut self, reverse: bool) {
        if self.modal().is_some() || self.order.is_empty() {
            return;
        }
        let current = self
            .active
            .and_then(|id| self.order.iter().position(|item| *item == id))
            .unwrap_or(0);
        let next = if reverse {
            (current + self.order.len() - 1) % self.order.len()
        } else {
            (current + 1) % self.order.len()
        };
        self.active = Some(self.order[next]);
    }

    pub(crate) fn capture(&mut self, id: WindowId) {
        self.modals.push((id, self.active));
        self.active = Some(id);
    }

    pub(crate) fn restore(&mut self) -> Option<WindowId> {
        let (closed, previous) = self.modals.pop()?;
        self.active = previous
            .filter(|id| self.modal() == Some(*id) || self.order.contains(id))
            .or_else(|| self.modal())
            .or_else(|| self.order.first().copied());
        Some(closed)
    }
}
