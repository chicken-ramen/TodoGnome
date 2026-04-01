use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Button, Label, Box as GtkBox, Orientation, ListBox, ListBoxRow, ScrolledWindow, DropTarget, Dialog, Entry, ComboBoxText, Calendar, Grid};
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
    
    fn add_todo(&self, item: TodoItem) -> Result<()> {
        let event = TodoEvent::TodoAdded(item);
        self.event_store.append_event(&event)?;
        
        // Refresh today's items
        let new_items = self.event_store.get_today_tasks()?;
        *self.today_items.borrow_mut() = new_items;
        
        // Update UI if list_box exists
        if let Some(list_box) = self.list_box.borrow().as_ref() {
            // Clear existing rows
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }
            self.populate_list(list_box);
        }
        
        Ok(())
    }
    
    fn refresh_list(&self) -> Result<()> {
        let new_items = self.event_store.get_today_tasks()?;
        *self.today_items.borrow_mut() = new_items;
        
        if let Some(list_box) = self.list_box.borrow().as_ref() {
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }
            self.populate_list(list_box);
        }
        Ok(())
    }
}

fn show_add_todo_dialog(parent: &ApplicationWindow) -> Option<TodoItem> {
    let dialog = Dialog::new_with_buttons(
        Some("Add New Todo"),
        Some(parent),
        gtk4::DialogFlags::MODAL,
        &[("Cancel", gtk4::ResponseType::Cancel), ("Add", gtk4::ResponseType::Accept)]
    );
    
    let content_area = dialog.content_area();
    let grid = Grid::new();
    grid.set_row_spacing(6);
    grid.set_column_spacing(12);
    grid.set_margin_all(12);
    
    // Title
    let title_label = Label::new(Some("Title:"));
    title_label.set_halign(gtk4::Align::Start);
    let title_entry = Entry::new();
    title_entry.set_hexpand(true);
    grid.attach(&title_label, 0, 0, 1, 1);
    grid.attach(&title_entry, 1, 0, 2, 1);
    
    // Priority
    let priority_label = Label::new(Some("Priority:"));
    priority_label.set_halign(gtk4::Align::Start);
    let priority_combo = ComboBoxText::new();
    priority_combo.append_text("Low");
    priority_combo.append_text("Medium");
    priority_combo.append_text("High");
    priority_combo.append_text("Critical");
    priority_combo.set_active(Some(1)); // Default Medium
    grid.attach(&priority_label, 0, 1, 1, 1);
    grid.attach(&priority_combo, 1, 1, 2, 1);
    
    // Due date (simplified: checkbox for today)
    let due_label = Label::new(Some("Due today:"));
    due_label.set_halign(gtk4::Align::Start);
    let due_checkbox = gtk4::CheckButton::new();
    grid.attach(&due_label, 0, 2, 1, 1);
    grid.attach(&due_checkbox, 1, 2, 2, 1);
    
    // Source URL (optional)
    let url_label = Label::new(Some("Source URL:"));
    url_label.set_halign(gtk4::Align::Start);
    let url_entry = Entry::new();
    url_entry.set_placeholder_text(Some("https://..."));
    grid.attach(&url_label, 0, 3, 1, 1);
    grid.attach(&url_entry, 1, 3, 2, 1);
    
    content_area.append(&grid);
    
    dialog.show();
    
    let response = dialog.run();
    let result = if response == gtk4::ResponseType::Accept {
        let title = title_entry.text().to_string();
        if title.is_empty() {
            None
        } else {
            let priority = match priority_combo.active_text().as_deref() {
                Some("Low") => Priority::Low,
                Some("High") => Priority::High,
                Some("Critical") => Priority::Critical,
                _ => Priority::Medium,
            };
            
            let due_date = if due_checkbox.is_active() {
                Some(Utc::now())
            } else {
                None
            };
            
            let source_url = url_entry.text().to_string();
            let source_url = if source_url.is_empty() { None } else { Some(source_url) };
            
            Some(TodoItem::new(
                title,
                None, // description
                priority,
                due_date,
                vec![], // stakeholders
                source_url,
                vec![], // tags
            ))
        }
    } else {
        None
    };
    
    dialog.close();
    result
}

fn main() -> anyhow::Result<()> {
    let app = Application::builder()
        .application_id("com.github.todognome")
        .build();

    let todo_app = Rc::new(TodoApp::new()?);
    
    let todo_app_weak = Rc::downgrade(&todo_app);
    app.connect_activate(move |app| {
        if let Some(todo_app) = todo_app_weak.upgrade() {
            match todo_app.create_ui(app) {
                Ok(window) => window.present(),
                Err(e) => eprintln!("Failed to create UI: {}", e),
            }
        }
    });

    app.run();
    Ok(())
}