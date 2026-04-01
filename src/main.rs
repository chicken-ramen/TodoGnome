use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Button, Label, Box as GtkBox, Orientation, ListBox, ListBoxRow, ScrolledWindow};
use todognome::{EventStore, TodoItem, Priority, Stakeholder};
use std::rc::Rc;
use std::cell::RefCell;
use std::fs;

struct TodoApp {
    event_store: Rc<EventStore>,
    today_items: RefCell<Vec<TodoItem>>,
}

impl TodoApp {
    fn new() -> anyhow::Result<Self> {
        // Ensure data directory exists
        fs::create_dir_all("./data")?;
        
        let event_store = EventStore::new("./data");
        let today_items = event_store.get_today_tasks()?;
        
        Ok(Self {
            event_store: Rc::new(event_store),
            today_items: RefCell::new(today_items),
        })
    }
    
    fn create_ui(&self, app: &gtk4::Application) -> anyhow::Result<ApplicationWindow> {
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
        
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .build();
        
        let list_box = ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .build();
        
        self.populate_list(&list_box);
        
        scrolled.set_child(Some(&list_box));
        
        let add_button = Button::with_label("Add New Todo");
        add_button.add_css_class("suggested-action");
        
        let app_weak = Rc::downgrade(&self.event_store);
        add_button.connect_clicked(move |_| {
            if let Some(store) = app_weak.upgrade() {
                // TODO: Open add todo dialog
                println!("Add todo clicked");
            }
        });
        
        main_box.append(&header_label);
        main_box.append(&scrolled);
        main_box.append(&add_button);
        
        window.set_child(Some(&main_box));
        Ok(window)
    }
    
    fn populate_list(&self, list_box: &ListBox) {
        let items = self.today_items.borrow();
        for item in items.iter() {
            let row = self.create_todo_row(item);
            list_box.append(&row);
        }
    }
    
    fn create_todo_row(&self, item: &TodoItem) -> ListBoxRow {
        let row = ListBoxRow::new();
        let row_box = GtkBox::new(Orientation::Horizontal, 12);
        row_box.set_margin_all(8);
        
        let checkbox = gtk4::CheckButton::new();
        checkbox.set_active(item.completed);
        
        let title_label = Label::new(Some(&item.title));
        title_label.set_hexpand(true);
        title_label.set_xalign(0.0);
        
        let due_label = if let Some(due) = item.due_date {
            Label::new(Some(&format!("Due: {}", due.format("%Y-%m-%d"))))
        } else {
            Label::new(Some("No due date"))
        };
        due_label.add_css_class("dim-label");
        
        let priority_label = Label::new(Some(match item.priority {
            Priority::Low => "🔵 Low",
            Priority::Medium => "🟡 Medium",
            Priority::High => "🟠 High",
            Priority::Critical => "🔴 Critical",
        }));
        
        row_box.append(&checkbox);
        row_box.append(&title_label);
        row_box.append(&due_label);
        row_box.append(&priority_label);
        
        row.set_child(Some(&row_box));
        row
    }
}

fn main() -> anyhow::Result<()> {
    let app = Application::builder()
        .application_id("com.github.todognome")
        .build();

    let todo_app = TodoApp::new()?;
    
    app.connect_activate(move |app| {
        match todo_app.create_ui(app) {
            Ok(window) => window.present(),
            Err(e) => eprintln!("Failed to create UI: {}", e),
        }
    });

    app.run();
    Ok(())
}