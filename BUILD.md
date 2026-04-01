# Building TodoGnome

## Prerequisites

### Rust Toolchain
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### GNOME Development Libraries
On Fedora:
```bash
sudo dnf install gtk4-devel glib2-devel
```

On Ubuntu/Debian:
```bash
sudo apt install libgtk-4-dev libglib2.0-dev
```

## Building

```bash
cargo build --release
```

The binary will be at `target/release/todognome`.

## Running

```bash
cargo run
```

Or run the built binary directly:
```bash
./target/release/todognome
```

## Installing the GNOME Shell Extension

```bash
cd extension/todognome@github
./install.sh
```

Then restart GNOME Shell (Alt+F2, type 'restart') and enable the extension using the GNOME Extensions app.

## Project Structure

- `src/main.rs` - Main application window and UI
- `src/lib.rs` - Event sourcing core, SQLite storage, todo models
- `extension/` - GNOME Shell extension for toolbar integration
- `data/` - SQLite database location (created automatically)

## Next Development Steps

1. Connect the add todo dialog to actually create todos
2. Implement checkbox completion with event sourcing
3. Add stakeholder and tag management UI
4. Improve due date picker with calendar widget
5. Implement drag-and-drop URL -> todo creation
6. Add search/filtering capabilities
7. Write unit tests for event store
8. Package for distribution (Flatpak, RPM)

## Architecture Notes

- Event-sourced: All state changes stored as immutable events in SQLite
- Materialized views: Current state derived from events for efficient querying
- GNOME integration: Shell extension reads status file written by Rust app
- GTK4: Modern GNOME toolkit with native look and feel