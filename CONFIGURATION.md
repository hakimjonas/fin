# Configuration

## Configuration File

The application reads its configuration from a TOML file located at `/usr/share/fin/config.toml`. You can customize this
file to suit your needs.

To modify the configuration file, you can use any text editor. For example, to edit the file using `nano`, you can run:

```sh
sudo nano /usr/share/fin/config.toml
```

Make sure to save your changes and restart the application for the new configuration to take effect.

Default Configuration

```toml
# Finë Default Configuration
title = "Finë"
columns = 2
stylesheet = "style.css"
use_system_theme = false

[default_commands]
# GNOME
gnome = [
    { label = "󰌾", command = "xdg-screensaver lock" },
    { label = "󰍃", command = "gnome-session-quit --logout" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]

# Hyprland
hyprland = [
    { label = "󰌾", command = "hyprctl dispatch exec hyprlock" },
    { label = "󰍃", command = "loginctl terminate-user $USER" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]

# Budgie
budgie = [
    { label = "󰌾", command = "xdg-screensaver lock" },
    { label = "󰍃", command = "budgie-session-quit --logout" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]

# XFCE
xfce = [
    { label = "󰌾", command = "xflock4" },
    { label = "󰍃", command = "xfce4-session-logout --logout" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]

# i3 Window Manager
i3 = [
    { label = "󰌾", command = "i3lock" },
    { label = "󰍃", command = "i3-msg exit" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]

# Sway (Wayland i3 Equivalent)
sway = [
    { label = "󰌾", command = "swaylock" },
    { label = "󰍃", command = "swaymsg exit" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]

# COSMIC DE (Pop!_OS)
cosmic = [
    { label = "󰌾", command = "system76-power suspend" },
    { label = "󰍃", command = "system76-power shutdown" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]

# Default (Fallback for unknown DEs)
default = [
    { label = "󰌾", command = "xdg-screensaver lock" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]
```

### Styling

The application uses a CSS file for styling, located at /usr/share/fin/style.css. You can customize this file to change
the appearance of the application. Here is the default style provided in style.css:

> **Note:** The default styling requires a Nerd Font to work correctly. You can download and install a Nerd Font
> from [Nerd Fonts](https://www.nerdfonts.com/). The Mononoki Nerd Font included with this application is released under
> the SIL Open Font License. For more details, see the [LICENSE](assets/font-licence/LICENSE.txt) file in this
> repository.


> **Note**: GTK4 does not support the full CSS specification. For more details on GTK4-specific CSS, refer to
> the [GTK4 CSS
documentation.](https://docs.gtk.org/gtk4/css-overview.html)

```css 
button {
    background-color: rgba(43, 53, 48, 0.6);
    color: rgba(242, 236, 228, 0.6);
    font-family: "Mononoki Nerd Font", monospace;
    font-size: 72px;
    border-radius: 7px;
    border: none;
    padding: 2px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    transition: background-color 0.3s ease, color 0.3s ease, box-shadow 0.3s ease, text-shadow 0.3s ease;
}

button:hover {
    background-color: rgba(107, 133, 124, 0.6);
    color: rgba(215, 211, 204, 0.8);
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
    text-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
}

button:focus {
    background-color: rgba(107, 133, 124, 0.6);
    color: rgba(242, 236, 228, 0.6);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.15);
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
}

button:focus:hover {
    background-color: rgba(112, 135, 124, 0.76);
    color: rgba(242, 236, 228, 0.6);
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.25);
    text-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
}

window {
    background-color: rgba(32, 44, 40, 0.3);
    border: 0 solid rgba(217, 140, 80, 0.3);
    padding: 2px;
    border-radius: 11px;
}

grid {
    padding: 1px;
}
```

### Using System Theme

If you prefer to use the system theme, you can set use_system_theme to true in the configuration file:

``` toml
use_system_theme = true
```  

When use_system_theme is set to true, the application will use the system's default GTK theme instead of the custom
style defined in style.css.

### Overwriting Configuration

To overwrite the default configuration, edit the config.toml file. For system-level changes, modify the file located at
`/usr/share/fin/config.toml`. For user-level changes, you can add your own config in your home directory, typically at `~
/.config/fin/config.toml`. But defer to your distribution's guidelines for the correct location.