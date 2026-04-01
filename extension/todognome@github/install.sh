#!/bin/bash

EXTENSION_DIR="$HOME/.local/share/gnome-shell/extensions/todognome@github"
SCHEMA_DIR="$HOME/.local/share/glib-2.0/schemas"

echo "Installing TodoGnome GNOME Shell extension..."

# Copy extension files
mkdir -p "$EXTENSION_DIR"
cp -r ./* "$EXTENSION_DIR/"

# Compile schemas
mkdir -p "$SCHEMA_DIR"
cp schemas/*.xml "$SCHEMA_DIR/"
glib-compile-schemas "$SCHEMA_DIR"

echo "Extension installed to $EXTENSION_DIR"
echo "Please restart GNOME Shell (Alt+F2, type 'restart') or log out and back in."
echo "Then enable the extension using GNOME Extensions app."