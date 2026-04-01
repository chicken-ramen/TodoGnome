use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Button, Label, Box as GtkBox, Orientation};

fn main() -> anyhow::Result<()> {
    let app = Application::builder()
        .application_id("com.github.todognome")
        .build();

    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("TodoGnome")
            .default_width(800)
            .default_height(600)
            .build();

        let main_box = GtkBox::new(Orientation::Vertical, 12);
        main_box.set_margin_all(12);

        let header_label = Label::new(Some("TodoGnome - Today's Tasks"));
        header_label.add_css_class("title-1");
        
        let todo_list = GtkBox::new(Orientation::Vertical, 6);
        todo_list.add_css_class("linked");
        
        let add_button = Button::with_label("Add New Todo");
        add_button.add_css_class("suggested-action");
        
        main_box.append(&header_label);
        main_box.append(&todo_list);
        main_box.append(&add_button);
        
        window.set_child(Some(&main_box));
        window.present();
    });

    app.run();
    Ok(())
}