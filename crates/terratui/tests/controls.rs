use terratui::terrabox::{Event, Key, KeyEvent, Modifiers, Rect, Size};
use terratui::widgets::{
    Button, FormDialog, Group, Label, Menu, MenuBar, MenuEntry, Panel, TabBehavior, TextInput,
    Tree, TreeNode,
};
use terratui::{Axis, CommandId, Desktop, Layout, Length, Widget};
fn key(key: Key) -> Event {
    Event::Key(KeyEvent::new(key, Modifiers::NONE))
}
fn group() -> (Group, terratui::WindowId, terratui::WindowId) {
    let mut group = Group::default();
    let label = group.add(Label("Name".into()), false);
    let input = group.add(TextInput::new(""), true);
    let button = group.add(Button::new("Save", CommandId(7)), true);
    group.layout = Layout::Split {
        axis: Axis::Vertical,
        children: vec![
            (Length::Fixed(1), Layout::Window(label)),
            (Length::Fixed(1), Layout::Window(input)),
            (Length::Fixed(1), Layout::Window(button)),
        ],
    };
    (group, input, button)
}
#[test]
fn nested_scopes_bubble_trap_and_restore_without_redraw_focus_changes() {
    let (mut inner, input, button) = group();
    inner.tab = TabBehavior::Bubble;
    let mut outer = Group::default();
    let inner_id = outer.add(inner, true);
    let other = outer.add(Button::new("Other", CommandId(8)), true);
    outer.layout = Layout::Split {
        axis: Axis::Horizontal,
        children: vec![
            (Length::Flex(1), Layout::Window(inner_id)),
            (Length::Flex(1), Layout::Window(other)),
        ],
    };
    let mut desktop = Desktop::default();
    let root = desktop.add_window(outer);
    let panel = desktop.add_window(Panel::new("Other window", vec![]));
    desktop.layout = Layout::Split {
        axis: Axis::Horizontal,
        children: vec![
            (Length::Flex(1), Layout::Window(root)),
            (Length::Flex(1), Layout::Window(panel)),
        ],
    };
    desktop.render(Size {
        width: 80,
        height: 20,
    });
    desktop.route(&key(Key::Tab));
    assert_eq!(
        desktop
            .widget_mut::<Group>(root)
            .unwrap()
            .widget_mut::<Group>(inner_id)
            .unwrap()
            .focused(),
        Some(button)
    );
    desktop.route(&key(Key::Tab));
    assert_eq!(
        desktop.widget_mut::<Group>(root).unwrap().focused(),
        Some(other)
    );
    desktop.route(&key(Key::BackTab));
    assert_eq!(
        desktop.widget_mut::<Group>(root).unwrap().focused(),
        Some(inner_id)
    );
    assert_eq!(
        desktop
            .widget_mut::<Group>(root)
            .unwrap()
            .widget_mut::<Group>(inner_id)
            .unwrap()
            .focused(),
        Some(button)
    );
    desktop
        .widget_mut::<Group>(root)
        .unwrap()
        .widget_mut::<Group>(inner_id)
        .unwrap()
        .focus(input);
    desktop
        .widget_mut::<Group>(root)
        .unwrap()
        .widget_mut::<Group>(inner_id)
        .unwrap()
        .widget_mut::<TextInput>(input)
        .unwrap()
        .accept_tab = true;
    desktop.route(&key(Key::Tab));
    assert_eq!(
        desktop
            .widget_mut::<Group>(root)
            .unwrap()
            .widget_mut::<Group>(inner_id)
            .unwrap()
            .widget_mut::<TextInput>(input)
            .unwrap()
            .text,
        "    "
    );
    for _ in 0..3 {
        desktop.render(Size {
            width: 80,
            height: 20,
        });
    }
    assert_eq!(desktop.focused(), Some(root));
    let (controls, input, button) = group();
    let modal = desktop.show_modal(
        FormDialog {
            title: "Form".into(),
            controls,
            accept: CommandId(9),
            cancel: CommandId(10),
        },
        Size {
            width: 30,
            height: 8,
        },
    );
    desktop.render(Size {
        width: 80,
        height: 20,
    });
    desktop.route(&key(Key::BackTab));
    assert_eq!(
        desktop
            .widget_mut::<FormDialog>(modal)
            .unwrap()
            .controls
            .focused(),
        Some(button)
    );
    desktop.route(&key(Key::Tab));
    assert_eq!(
        desktop
            .widget_mut::<FormDialog>(modal)
            .unwrap()
            .controls
            .focused(),
        Some(input)
    );
    assert_eq!(desktop.route(&key(Key::Enter)).command, Some(CommandId(9)));
    assert_eq!(
        desktop.route(&key(Key::Escape)).command,
        Some(CommandId(10))
    );
    assert!(desktop.has_modal()); // The application owns command actions.
    desktop.close_modal();
    assert_eq!(desktop.focused(), Some(root));
}
#[test]
fn text_input_deletes_graphemes_and_handles_external_text_replacement() {
    let mut input = TextInput::new("e\u{301}界👩‍💻");
    input.event(&key(Key::Backspace));
    assert_eq!(input.text, "e\u{301}界");
    input.event(&key(Key::Left));
    input.event(&key(Key::Backspace));
    assert_eq!(input.text, "界");
    input.text = "x".into();
    input.event(&key(Key::End));
    input.event(&key(Key::Char('λ')));
    assert_eq!(input.text, "xλ");
}
#[test]
fn dropdown_mnemonics_skip_disabled_items_and_dispatch_shortcuts() {
    let mut disabled = MenuEntry::command("&Disabled", CommandId(99));
    disabled.enabled = false;
    let mut action = MenuEntry::command("&Open", CommandId(7));
    action.shortcut = Some(KeyEvent::new(Key::Function(3), Modifiers::NONE));
    let mut desktop = Desktop::default();
    desktop.menu = MenuBar::dropdowns(vec![Menu {
        label: "&File".into(),
        entries: vec![disabled, MenuEntry::separator(), action],
    }]);
    assert_eq!(
        desktop.route(&key(Key::Function(3))).command,
        Some(CommandId(7))
    );
    desktop.route(&Event::Key(KeyEvent::new(Key::Char('f'), Modifiers::ALT)));
    assert!(desktop.menu.is_active());
    assert_eq!(desktop.route(&key(Key::Enter)).command, Some(CommandId(7)));
    desktop.route(&key(Key::Function(10)));
    desktop.route(&key(Key::Char('d')));
    assert!(desktop.menu.is_active());
    desktop.route(&key(Key::Escape));
    assert!(!desktop.menu.is_active());
}
#[test]
fn hidden_minimum_layouts_and_trees_clip_at_small_sizes() {
    let layout = Layout::Split {
        axis: Axis::Horizontal,
        children: vec![
            (Length::Fixed(50), Layout::Hidden),
            (
                Length::MinFlex { min: 4, weight: 1 },
                Layout::Window(terratui::WindowId(1)),
            ),
            (Length::Flex(1), Layout::Window(terratui::WindowId(2))),
        ],
    };
    for width in 0..30 {
        let areas = layout.arrange(Rect::new(0, 0, width, 4));
        assert_eq!(areas.len(), 2);
        assert_eq!(areas[0].1.width + areas[1].1.width, width);
        assert!(areas.iter().all(|(_, r)| r.right() <= width));
    }
    let mut tree = Tree::new(
        "Tree",
        vec![TreeNode {
            label: "root".into(),
            expanded: false,
            children: vec![TreeNode {
                label: "child".into(),
                expanded: false,
                children: vec![],
            }],
        }],
    );
    tree.event(&key(Key::Right));
    assert!(tree.nodes[0].expanded);
    tree.event(&key(Key::Right));
    assert_eq!(tree.selected_path(), Some(vec![0, 0]));
    tree.event(&key(Key::Left));
    assert_eq!(tree.selected_path(), Some(vec![0]));
    tree.event(&key(Key::Left));
    assert!(!tree.nodes[0].expanded);
    let mut desktop = Desktop::default();
    let id = desktop.add_window(tree);
    desktop.layout = Layout::Window(id);
    for width in 0..8 {
        for height in 0..5 {
            desktop.render(Size { width, height });
        }
    }
}

#[test]
fn mouse_hit_testing_does_not_activate_focused_button_when_label_is_clicked() {
    use terratui::terrabox::{MouseButton, MouseEvent, MouseKind, Position, Surface};
    let (mut group, _, button) = group();
    group.focus(button);
    let mut surface = Surface::new(Size {
        width: 20,
        height: 5,
    });
    group.draw(
        &mut surface.canvas(surface.bounds()),
        true,
        &terratui::Theme::default(),
    );
    let click = |y| {
        Event::Mouse(MouseEvent {
            position: Position { x: 1, y },
            button: MouseButton::Left,
            kind: MouseKind::Press,
            modifiers: Modifiers::NONE,
        })
    };
    assert_eq!(group.event(&click(0)).command, None);
    assert_eq!(group.focused(), Some(button));
    assert_eq!(group.event(&click(2)).command, Some(CommandId(7)));
    group.tab = TabBehavior::Manual;
    assert!(!group.event(&key(Key::Tab)).consumed);
}
