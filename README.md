# TodoGnome

A GNOME desktop application for managing todo items with event sourcing, toolbar integration, and drag-and-drop support.

## Features

- **Event-sourced architecture**: All changes are stored as immutable events
- **GNOME integration**: Toolbar/status icon for quick access to today's tasks
- **Drag-and-drop**: Drop links from Teams, email, browsers, etc.
- **Rich todo items**: Due dates, priorities, stakeholders, tags
- **Today's tasks view**: Quick access to tasks due today
- **SQLite-backed storage**: Events stored in SQLite with materialized views

## Building

Make sure you have Rust and Cargo installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Install GNOME development dependencies (on Fedora):

```bash
sudo dnf install gtk4-devel glib2-devel
```

Build and run:

```bash
cargo run
```

## Architecture

The application uses event sourcing:
- All state changes are stored as events in JSON files
- Current state can be rebuilt by replaying events
- Events are immutable and append-only

## Roadmap

- [x] Basic Rust + GTK4 setup
- [x] Main window with todo list
- [x] Event store implementation (SQLite)
- [x] GNOME Shell extension skeleton
- [x] Drag-and-drop target
- [x] Today's tasks filtering
- [ ] Add todo dialog (in progress)
- [ ] Checkbox completion
- [ ] Stakeholder and tag UI
- [ ] Due date calendar picker
- [ ] Package for distribution

See [BUILD.md](BUILD.md) for detailed build instructions and next steps.