use gio::prelude::*;
use gtk::prelude::*;

fn main() {
    let application = gtk::Application::new(
        Some("com.github.gtk-ui-rs"),
        gio::ApplicationFlags::from_bits_truncate(4)
    ).expect("Initialization failed...");

    application.connect_activate(|_| {});
    application.connect_open(move |app, files, _| {
        let path = files[0].get_path().unwrap();
        build_ui(app, &path);
    });

    application.run(&std::env::args().collect::<Vec<_>>());
}

fn build_ui(application: &gtk::Application, file_path: &std::path::Path) {
    let window = gtk::ApplicationWindow::new(application);
    let drawing_area = Box::new(gtk::DrawingArea::new)();

    let mut opt = usvg::Options::default();
    opt.path = Some(file_path.into());

    let tree = usvg::Tree::from_file(file_path, &opt).unwrap();

    drawing_area.connect_draw(move |w, cr| {
        let s = usvg::ScreenSize::new(
            w.get_allocated_width() as u32,
            w.get_allocated_height() as u32,
        ).unwrap();
        resvg_cairo::render_to_canvas(&tree, s, cr);

        Inhibit(false)
    });

    window.set_default_size(500, 500);

    let window_clone = window.clone();
    window.connect_delete_event(move |_, _| {
        window_clone.destroy();
        Inhibit(false)
    });
    window.add(&drawing_area);
    window.show_all();
}
