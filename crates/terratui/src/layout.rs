use crate::WindowId;
use terrabox::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Length {
    Fixed(u16),
    Flex(u16),
    /// Reserve a minimum, then share remaining space by weight.
    MinFlex {
        min: u16,
        weight: u16,
    },
}

/// Nested splits. Fixed lengths truncate at the edge; flex lengths share the rest.
#[derive(Clone, Debug)]
pub enum Layout {
    /// Collapsed layouts consume no space and cannot receive focus.
    Hidden,
    Window(WindowId),
    Split {
        axis: Axis,
        children: Vec<(Length, Layout)>,
    },
}

impl Default for Layout {
    fn default() -> Self {
        Self::Split {
            axis: Axis::Horizontal,
            children: Vec::new(),
        }
    }
}

impl Layout {
    pub fn arrange(&self, area: Rect) -> Vec<(WindowId, Rect)> {
        let mut result = Vec::new();
        self.assign(area, &mut result);
        result
    }

    fn assign(&self, area: Rect, result: &mut Vec<(WindowId, Rect)>) {
        match self {
            Self::Hidden => {}
            Self::Window(id) => result.push((*id, area)),
            Self::Split { axis, children } => {
                let total = match axis {
                    Axis::Horizontal => area.width,
                    Axis::Vertical => area.height,
                };
                let children: Vec<_> = children
                    .iter()
                    .filter(|(_, layout)| !matches!(layout, Layout::Hidden))
                    .collect();
                let fixed: u64 = children
                    .iter()
                    .map(|(length, _)| match length {
                        Length::Fixed(value) => u64::from(*value),
                        Length::MinFlex { min, .. } => u64::from(*min),
                        _ => 0,
                    })
                    .sum();
                let mut weight: u64 = children
                    .iter()
                    .map(|(length, _)| match length {
                        Length::Flex(value) => u64::from(*value),
                        Length::MinFlex { weight, .. } => u64::from(*weight),
                        _ => 0,
                    })
                    .sum();
                let mut flexible = u64::from(total).saturating_sub(fixed);
                let mut used = 0;
                for (length, child) in children {
                    let desired = match length {
                        Length::Fixed(value) => *value,
                        Length::Flex(value) | Length::MinFlex { weight: value, .. } => {
                            let share = (flexible * u64::from(*value))
                                .checked_div(weight)
                                .unwrap_or(0);
                            weight -= u64::from(*value);
                            flexible -= share;
                            let min = if let Length::MinFlex { min, .. } = length {
                                *min
                            } else {
                                0
                            };
                            min.saturating_add(share as u16)
                        }
                    };
                    let size = desired.min(total.saturating_sub(used));
                    let rect = match axis {
                        Axis::Horizontal => {
                            Rect::new(area.x.saturating_add(used), area.y, size, area.height)
                        }
                        Axis::Vertical => {
                            Rect::new(area.x, area.y.saturating_add(used), area.width, size)
                        }
                    };
                    child.assign(rect, result);
                    used += size;
                }
            }
        }
    }
}
