#![cfg(feature = "kitty-graphics")]
use std::sync::Arc;
use terratui::terrabox::{Rect, Size, Surface, graphics::RgbaImage};
use terratui::widgets::{Dialog, Group, ImageView};
use terratui::{Axis, CommandId, Desktop, Layout, Length, Theme, Widget};
#[test]
fn image_widgets_use_clipped_layout_regions_and_have_a_real_text_fallback() {
    let data = Arc::new(RgbaImage::new(2, 2, vec![255; 16]).unwrap());
    let mut view = ImageView::new("Preview", Some(data));
    let mut surface = Surface::new(Size {
        width: 60,
        height: 12,
    });
    view.draw(
        &mut surface.canvas(surface.bounds()),
        false,
        &Theme::default(),
    );
    assert_eq!(surface.cell(2, 2).unwrap().text(), "T"); // Text fallback is visible before any confirmation.
    let mut requests = Vec::new();
    view.collect_graphics(Rect::new(2, 3, 20, 8), &mut requests);
    assert!(requests.is_empty());
    view.prepare_graphics(true);
    view.collect_graphics(Rect::new(2, 3, 20, 8), &mut requests);
    assert_eq!(requests[0].area, Rect::new(3, 4, 18, 6));
    view.disabled = true;
    view.prepare_graphics(true);
    requests.clear();
    view.collect_graphics(Rect::new(0, 0, 20, 8), &mut requests);
    assert!(requests.is_empty());
}
#[test]
fn nested_image_requests_use_the_same_positions_as_text_layouts() {
    let image = Arc::new(RgbaImage::new(1, 1, vec![0; 4]).unwrap());
    let mut group = Group::default();
    let left = group.add(ImageView::new("A", Some(image.clone())), true);
    let right = group.add(ImageView::new("B", Some(image)), true);
    group.layout = Layout::Split {
        axis: Axis::Horizontal,
        children: vec![
            (Length::Fixed(10), Layout::Window(left)),
            (Length::Flex(1), Layout::Window(right)),
        ],
    };
    let mut surface = Surface::new(Size {
        width: 30,
        height: 10,
    });
    group.prepare_graphics(true);
    group.draw(
        &mut surface.canvas(surface.bounds()),
        true,
        &Theme::default(),
    );
    let mut requests = Vec::new();
    group.collect_graphics(Rect::new(4, 5, 30, 10), &mut requests);
    assert_eq!(requests[0].area, Rect::new(5, 6, 8, 8));
    assert_eq!(requests[1].area, Rect::new(15, 6, 18, 8));
    group.layout = Layout::Window(left);
    group.draw(
        &mut surface.canvas(surface.bounds()),
        true,
        &Theme::default(),
    );
    requests.clear();
    group.collect_graphics(Rect::new(4, 5, 30, 10), &mut requests);
    assert_eq!(requests.len(), 1);
}

#[test]
fn dialogs_menus_and_hidden_panels_remove_requests_from_the_scene() {
    let mut view = ImageView::new(
        "Preview",
        Some(Arc::new(RgbaImage::new(1, 1, vec![255; 4]).unwrap())),
    );
    view.prepare_graphics(true);
    let mut desktop = Desktop::default();
    let id = desktop.add_window(view);
    desktop.layout = Layout::Window(id);
    let size = Size {
        width: 40,
        height: 12,
    };
    desktop.render(size);
    assert_eq!(desktop.image_requests().len(), 1);
    desktop.show_modal(
        Dialog {
            title: "Help".into(),
            message: "Overlay".into(),
            accept: CommandId(1),
        },
        Size {
            width: 20,
            height: 5,
        },
    );
    desktop.render(size);
    assert!(desktop.image_requests().is_empty());
    desktop.close_modal();
    desktop.render(size);
    assert_eq!(desktop.image_requests().len(), 1);
    desktop.menu = terratui::widgets::MenuBar::new(vec![("Help".into(), CommandId(1))]);
    desktop.menu.event(&terratui::terrabox::Event::Key(
        terratui::terrabox::KeyEvent::new(
            terratui::terrabox::Key::Function(10),
            terratui::terrabox::Modifiers::NONE,
        ),
    ));
    desktop.render(size);
    assert!(desktop.image_requests().is_empty());
    desktop.menu.close();
    desktop.layout = Layout::Hidden;
    desktop.render(size);
    assert!(desktop.image_requests().is_empty());
}
