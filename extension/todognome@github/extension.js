import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import St from 'gi://St';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const StatusFile = GLib.get_user_cache_dir() + '/todognome/status.json';
const AppID = 'com.github.todognome';

export default class TodoGnomeExtension {
    constructor() {
        this._indicator = null;
        this._statusLabel = null;
        this._timeoutId = null;
        this._statusData = { today_count: 0, critical_count: 0, high_count: 0 };
    }

    enable() {
        this._indicator = new PanelMenu.Button(0.0, 'TodoGnome', false);
        
        let icon = new St.Icon({
            icon_name: 'task-due-symbolic',
            style_class: 'system-status-icon'
        });
        
        this._statusLabel = new St.Label({
            text: '0',
            y_align: St.Align.MIDDLE
        });
        
        let box = new St.BoxLayout({ style_class: 'panel-status-menu-box' });
        box.add_child(icon);
        box.add_child(this._statusLabel);
        this._indicator.add_child(box);
        
        // Create popup menu
        let menu = this._indicator.menu;
        let todayItem = new PopupMenu.PopupMenuItem("Today's Tasks");
        todayItem.connect('activate', () => this._launchApp());
        menu.addMenuItem(todayItem);
        
        menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        
        let addItem = new PopupMenu.PopupMenuItem("Add New Todo");
        addItem.connect('activate', () => this._launchApp(['--add']));
        menu.addMenuItem(addItem);
        
        let openItem = new PopupMenu.PopupMenuItem("Open TodoGnome");
        openItem.connect('activate', () => this._launchApp());
        menu.addMenuItem(openItem);
        
        Main.panel.addToStatusArea('todognome-indicator', this._indicator);
        
        // Start monitoring status file
        this._updateStatus();
        this._timeoutId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 30, () => {
            this._updateStatus();
            return GLib.SOURCE_CONTINUE;
        });
    }
    
    disable() {
        if (this._timeoutId) {
            GLib.source_remove(this._timeoutId);
            this._timeoutId = null;
        }
        
        if (this._indicator) {
            this._indicator.destroy();
            this._indicator = null;
        }
        this._statusLabel = null;
    }
    
    _updateStatus() {
        try {
            let file = Gio.File.new_for_path(StatusFile);
            if (!file.query_exists(null)) {
                this._statusData = { today_count: 0, critical_count: 0, high_count: 0 };
            } else {
                let [success, contents] = file.load_contents(null);
                if (success) {
                    let data = JSON.parse(new TextDecoder().decode(contents));
                    this._statusData = data;
                }
            }
            
            if (this._statusLabel) {
                let count = this._statusData.today_count || 0;
                this._statusLabel.text = count.toString();
                
                // Update icon style based on priority
                if (this._statusData.critical_count > 0) {
                    this._statusLabel.add_style_class_name('todognome-critical');
                    this._statusLabel.remove_style_class_name('todognome-high');
                    this._statusLabel.remove_style_class_name('todognome-normal');
                } else if (this._statusData.high_count > 0) {
                    this._statusLabel.add_style_class_name('todognome-high');
                    this._statusLabel.remove_style_class_name('todognome-critical');
                    this._statusLabel.remove_style_class_name('todognome-normal');
                } else {
                    this._statusLabel.add_style_class_name('todognome-normal');
                    this._statusLabel.remove_style_class_name('todognome-critical');
                    this._statusLabel.remove_style_class_name('todognome-high');
                }
            }
        } catch (e) {
            logError(e, 'TodoGnome extension error');
        }
    }
    
    _launchApp(args = []) {
        try {
            let app = Gio.AppInfo.create_from_commandline('todognome', 'TodoGnome', Gio.AppInfoCreateFlags.NONE);
            app.launch(args, null);
        } catch (e) {
            logError(e, 'Failed to launch TodoGnome');
        }
    }
}