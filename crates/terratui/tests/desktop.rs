use terratui::terrabox::{Event, Key, KeyEvent, Modifiers, Rect, Size};
use terratui::widgets::{Dialog, MenuBar, Panel};
use terratui::{Axis, CommandId, Desktop, Layout, Length, Scope, WindowId};

fn key(key: Key) -> Event {
    Event::Key(KeyEvent::new(key, Modifiers::NONE))
}

fn desktop() -> (Desktop, WindowId, WindowId) {
    let mut desktop = Desktop::default();
    let left = desktop.add_window(Panel::new("Left", vec!["first".into(), "second".into()]));
    let right = desktop.add_window(Panel::new("Right", vec!["界 e\u{301}".into()]));
    desktop.layout = Layout::Split {
        axis: Axis::Horizontal,
        children: vec![
            (Length::Flex(1), Layout::Window(left)),
            (Length::Flex(1), Layout::Window(right)),
        ],
    };
    (desktop, left, right)
}

fn modal(desktop: &mut Desktop) -> WindowId {
    desktop.show_modal(
        Dialog {
            title: "Dialog".into(),
            message: "Modal".into(),
            accept: CommandId(7),
        },
        Size {
            width: 20,
            height: 5,
        },
    )
}

#[test]
fn focus_cycles_and_nested_modals_restore_the_previous_target() {
    let (mut desktop, left, right) = desktop();
    assert_eq!(desktop.focused(), Some(left));
    desktop.route(&key(Key::Tab));
    assert_eq!(desktop.focused(), Some(right));
    let first = modal(&mut desktop);
    let second = modal(&mut desktop);
    desktop.route(&key(Key::Tab));
    assert_eq!(desktop.focused(), Some(second));
    assert!(!desktop.focus(left));
    desktop.route(&key(Key::Escape));
    assert_eq!(desktop.focused(), Some(first));
    desktop.route(&key(Key::Escape));
    assert_eq!(desktop.focused(), Some(right));
    desktop.route(&key(Key::BackTab));
    assert_eq!(desktop.focused(), Some(left));
}

#[test]
fn local_commands_override_global_and_modals_capture_global_shortcuts() {
    let (mut desktop, left, _) = desktop();
    let bound = KeyEvent::new(Key::Function(2), Modifiers::NONE);
    desktop.bindings.bind(Scope::Global, bound, CommandId(1));
    desktop
        .bindings
        .bind(Scope::Window(left), bound, CommandId(2));
    assert_eq!(
        desktop.route(&Event::Key(bound)).command,
        Some(CommandId(2))
    );
    desktop.route(&key(Key::Tab));
    assert_eq!(
        desktop.route(&Event::Key(bound)).command,
        Some(CommandId(1))
    );
    modal(&mut desktop);
    let result = desktop.route(&Event::Key(bound));
    assert!(result.consumed);
    assert_eq!(result.command, None);
    assert_eq!(desktop.route(&key(Key::Enter)).command, Some(CommandId(7)));
}

#[test]
fn menu_dispatches_the_same_command_identity_as_shortcuts() {
    let (mut desktop, _, _) = desktop();
    desktop.menu = MenuBar::new(vec![
        ("About".into(), CommandId(1)),
        ("Quit".into(), CommandId(2)),
    ]);
    desktop.route(&key(Key::Function(10)));
    desktop.route(&key(Key::Right));
    assert_eq!(desktop.route(&key(Key::Enter)).command, Some(CommandId(2)));
    assert!(!desktop.menu.is_active());
    desktop.route(&key(Key::Function(10)));
    desktop.route(&key(Key::Escape));
    assert!(!desktop.menu.is_active());
}

#[test]
fn nested_layouts_cover_the_area_without_overflow_at_tiny_sizes() {
    let layout = Layout::Split {
        axis: Axis::Vertical,
        children: vec![
            (Length::Fixed(2), Layout::Window(WindowId(1))),
            (
                Length::Flex(1),
                Layout::Split {
                    axis: Axis::Horizontal,
                    children: vec![
                        (Length::Flex(1), Layout::Window(WindowId(2))),
                        (Length::Flex(2), Layout::Window(WindowId(3))),
                    ],
                },
            ),
        ],
    };
    for width in 0..20 {
        for height in 0..10 {
            let regions = layout.arrange(Rect::new(0, 0, width, height));
            assert!(
                regions
                    .iter()
                    .all(|(_, rect)| rect.right() <= width && rect.bottom() <= height)
            );
            assert_eq!(regions[1].1.width + regions[2].1.width, width);
            assert_eq!(regions[0].1.height + regions[1].1.height, height);
        }
    }
}

#[test]
fn resize_and_modal_removal_recompose_without_stale_cells() {
    let (mut desktop, _, _) = desktop();
    let size = Size {
        width: 60,
        height: 12,
    };
    let before = desktop.render(size);
    modal(&mut desktop);
    let overlay = desktop.render(size);
    assert_ne!(before.cell(21, 4), overlay.cell(21, 4));
    desktop.close_modal();
    let restored = desktop.render(size);
    for y in 0..size.height {
        for x in 0..size.width {
            assert_eq!(before.cell(x, y), restored.cell(x, y));
        }
    }
    for width in 0..8 {
        for height in 0..5 {
            desktop.render(Size { width, height });
        }
    }
}

#[test]
fn widgets_clip_titles_and_preserve_combining_clusters() {
    let (mut desktop, _, _) = desktop();
    desktop.route(&key(Key::Tab));
    let frame = desktop.render(Size {
        width: 20,
        height: 5,
    });
    assert_eq!(frame.cell(11, 1).unwrap().text(), "界");
    assert!(frame.cell(12, 1).is_none());
    assert_eq!(frame.cell(14, 1).unwrap().text(), "e\u{301}");
    assert_eq!(frame.cell(19, 1).unwrap().text(), "│");
}

#[test]
fn retained_widgets_can_be_updated_and_modal_bindings_are_scoped() {
    let (mut desktop, left, _) = desktop();
    desktop.widget_mut::<Panel>(left).unwrap().lines = vec!["updated".into()];
    let frame = desktop.render(Size {
        width: 40,
        height: 8,
    });
    assert_eq!(frame.cell(1, 1).unwrap().text(), "u");
    assert!(desktop.widget_mut::<Dialog>(left).is_none());
    let modal = modal(&mut desktop);
    let bound = KeyEvent::new(Key::Function(2), Modifiers::NONE);
    desktop.bindings.bind(Scope::Global, bound, CommandId(1));
    desktop
        .bindings
        .bind(Scope::Window(modal), bound, CommandId(9));
    assert_eq!(
        desktop.route(&Event::Key(bound)).command,
        Some(CommandId(9))
    );
}
