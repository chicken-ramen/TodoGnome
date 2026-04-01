use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Button, Label, Box as GtkBox, Orientation, ListBox, ListBoxRow, ScrolledWindow, DropTarget};
use gtk4::gdk;
use gtk4::gio;
use anyhow::Result;
use chrono::Utc;
use todognome::{EventStore, TodoItem, Priority, Stakeholder, TodoEvent};
use std::rc::Rc;
use std::cell::RefCell;
use std::fs;

struct TodoApp {
    event_store: Rc<EventStore>,
    today_items: RefCell<Vec<TodoItem>>,
    list_box: RefCell<Option<ListBox>>,
}

impl TodoApp {
    fn new() -> anyhow::Result<Self> {
        // Ensure data directory exists
        fs::create_dir_all("./data")?;
        
        let event_store = EventStore::new("./data");
        let today_items = event_store.get_today_tasks()?;
        
        // Write initial status file for GNOME Shell extension
        let _ = event_store.write_status_file();
        
        Ok(Self {
            event_store: Rc::new(event_store),
            today_items: RefCell::new(today_items),
            list_box: RefCell::new(None),
        })
    }
    
    fn create_ui(&self, app: &gtk4::Application) -> anyhow::Result<ApplicationWindow> {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("TodoGnome")
            .default_width(800)
            .default_height(600)
            .build();

        // Set up drag-and-drop target for URLs from other apps
        let formats = gdk::ContentFormats::new_for_gtype(glib::types::Type::STRING);
        let drop_target = DropTarget::new(Some(&formats), gdk::DragAction::COPY);
        
        let event_store_weak = Rc::downgrade(&self.event_store);
        drop_target.connect_drop(move |_, value, _, _| {
            if let Some(text) = value.get::<String>() {
                // Extract URLs from dropped text
                let urls: Vec<String> = text.lines()
                    .filter(|line| line.starts_with("http://") || line.starts_with("https://"))
                    .map(|s| s.trim().to_string())
                    .collect();
                
                for url in urls {
                    println!("Dropped URL: {}", url);
                    // TODO: Create todo item from URL
                    if let Some(store) = event_store_weak.upgrade() {
                        // For now, just print
                        println!("Would create todo for URL: {}", url);
                    }
                }
                true
            } else {
                false
            }
        });
        window.add_controller(&drop_target);

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
        
        // Store reference to list box for later updates
        *self.list_box.borrow_mut() = Some(list_box.clone());
        
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