Configuration

Finë reads its configuration from a TOML file. By default, the system-wide configuration is located at /usr/share/fin/config.toml. You can customize this file to tailor Finë’s behavior. For system-level changes, edit this file directly. For user-level customizations, you can also place a configuration file at ~/.config/fin/config.toml (subject to your distribution’s guidelines).
Editing the Configuration File

For example, to edit the configuration using nano:

```sh
sudo nano /usr/share/fin/config.toml
```

After saving changes, restart Finë to apply the new configuration.

Default Configuration

```toml
# Finë Default Configuration
title = "Finë"
use_gtk_theme = false
theme = "default"
stylesheet = "style.css"

[layout]
window_width_ratio = 0.3  # 30% of the screen width
window_height_ratio = 0.3 # 30% of the screen height
button_font_ratio = 0.12  # 12% of the screen height

[default_commands]

# Hyprland: Locks screen, logs out, reboots, and powers off.
[default_commands.hyprland]
columns = 2
buttons = [
    { label = "󰌾", command = "hyprlock", css_classes = ["lock-button"], widget_name = "lock-button" },
    { label = "󰍃", command = "hyprctl dispatch exit", css_classes = ["logout-button"], widget_name = "logout-button" },
    { label = "󰜉", command = "systemctl reboot", css_classes = ["reboot-button"], widget_name = "reboot-button" },
    { label = "󰐥", command = "systemctl poweroff", css_classes = ["poweroff-button"], widget_name = "poweroff-button" }
]

# Sway: Locks screen, logs out, reboots, and powers off.
[default_commands.sway]
columns = 2
buttons = [
    { label = "󰌾", command = "swaylock -f -c 000000", css_classes = ["lock-button"], widget_name = "lock-button" },
    { label = "󰍃", command = "swaymsg exit", css_classes = ["logout-button"], widget_name = "logout-button" },
    { label = "󰜉", command = "systemctl reboot", css_classes = ["reboot-button"], widget_name = "reboot-button" },
    { label = "󰐥", command = "systemctl poweroff", css_classes = ["poweroff-button"], widget_name = "poweroff-button" }
]

# i3: Locks screen, logs out, reboots, and powers off.
[default_commands.i3]
columns = 2
buttons = [
    { label = "󰌾", command = "i3lock -c 000000", css_classes = ["lock-button"], widget_name = "lock-button" },
    { label = "󰍃", command = "i3-msg exit", css_classes = ["logout-button"], widget_name = "logout-button" },
    { label = "󰜉", command = "systemctl reboot", css_classes = ["reboot-button"], widget_name = "reboot-button" },
    { label = "󰐥", command = "systemctl poweroff", css_classes = ["poweroff-button"], widget_name = "poweroff-button" }
]

# GNOME: Locks screen, logs out, reboots, and powers off.
[default_commands.gnome]
columns = 2
buttons = [
    { label = "󰌾", command = "dbus-send --type=method_call --dest=org.gnome.ScreenSaver /org/gnome/ScreenSaver org.gnome.ScreenSaver.Lock", css_classes = ["lock-button"], widget_name = "lock-button" },
    { label = "󰍃", command = "gnome-session-quit --logout", css_classes = ["logout-button"], widget_name = "logout-button" },
    { label = "󰜉", command = "systemctl reboot", css_classes = ["reboot-button"], widget_name = "reboot-button" },
    { label = "󰐥", command = "systemctl poweroff", css_classes = ["poweroff-button"], widget_name = "poweroff-button" }
]

# Budgie: Locks screen, logs out, reboots, and powers off.
[default_commands.budgie]
columns = 2
buttons = [
    { label = "󰌾", command = "budgie-screensaver lock", css_classes = ["lock-button"], widget_name = "lock-button" },
    { label = "󰍃", command = "budgie-session-quit --logout", css_classes = ["logout-button"], widget_name = "logout-button" },
    { label = "󰜉", command = "systemctl reboot", css_classes = ["reboot-button"], widget_name = "reboot-button" },
    { label = "󰐥", command = "systemctl poweroff", css_classes = ["poweroff-button"], widget_name = "poweroff-button" }
]

# XFCE: Locks screen, logs out, reboots, and powers off.
[default_commands.xfce]
columns = 2
buttons = [
    { label = "󰌾", command = "xflock4", css_classes = ["lock-button"], widget_name = "lock-button" },
    { label = "󰍃", command = "xfce4-session-logout --logout", css_classes = ["logout-button"], widget_name = "logout-button" },
    { label = "󰜉", command = "systemctl reboot", css_classes = ["reboot-button"], widget_name = "reboot-button" },
    { label = "󰐥", command = "systemctl poweroff", css_classes = ["poweroff-button"], widget_name = "poweroff-button" }
]

# COSMIC: Suspends, locks screen, logs out, reboots, and powers off.
[default_commands.cosmic]
columns = 2
buttons = [
    { label = "󰤄", command = "systemctl suspend", css_classes = ["suspend-button"], widget_name = "suspend-button" },
    { label = "󰌾", command = "loginctl lock-session", css_classes = ["lock-button"], widget_name = "lock-button" },
    { label = "󰍃", command = "loginctl terminate-session $XDG_SESSION_ID", css_classes = ["logout-button"], widget_name = "logout-button" },
    { label = "󰜉", command = "systemctl reboot", css_classes = ["reboot-button"], widget_name = "reboot-button" },
    { label = "󰐥", command = "systemctl poweroff", css_classes = ["poweroff-button"], widget_name = "poweroff-button" }
]

# Common commands (fallback)
[default_commands.common]
columns = 2
buttons = [
    { label = "Lock", command = "loginctl lock-session", css_classes = ["lock-button"], widget_name = "lock-button" },
    { label = "Logout", command = "loginctl terminate-session $XDG_SESSION_ID", css_classes = ["logout-button"], widget_name = "logout-button" },
    { label = "Reboot", command = "systemctl reboot", css_classes = ["reboot-button"], widget_name = "reboot-button" },
    { label = "Poweroff", command = "systemctl poweroff", css_classes = ["poweroff-button"], widget_name = "poweroff-button" }
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
/* Finë Modern Default Stylesheet
   This stylesheet uses CSS variables for theming.
   These variables are injected dynamically from the theme.toml file (via Rust code).

   The following CSS custom properties are defined:

   Palette Variables:
     --palette0           : from palette0 (Base)
     --palette1           : from palette1 (Surface)
     --palette2           : from palette2 (Overlay)
     --palette3           : from palette3 (Muted)
     --palette4           : from palette4 (Subtle)
     --palette5           : from palette5 (Text)
     --palette6           : from palette6 (Love)
     --palette7           : from palette7 (Gold)
     --palette8           : from palette8 (Rose)
     --palette9           : from palette9 (Pine)
     --palette10          : from palette10 (Foam)
     --palette11          : from palette11 (Iris)
     --palette12          : from palette12 (Highlight Low)
     --palette13          : from palette13 (Highlight Med)
     --palette14          : from palette14 (Highlight High)
     --palette15          : from palette15 (Highlight Text)

   UI-Specific Colors:
     --background         : from background (Window background)
     --foreground         : from foreground (Window foreground)

   Button Colors:
     --button-normal-background : from button-normal-background (Normal button background)
     --button-normal-text       : from button-normal-text (Normal button text color)
     --button-focus-background  : from button-focus-background (Button focus background)
     --button-focus-text        : from button-focus-text (Button focus text color)
     --button-hover-background  : from button-hover-background (Button hover background)
     --button-hover-text        : from button-hover-text (Button hover text)

   Ensure that your theme.toml file defines all these keys accordingly.
*/

/* Finë Modern Default Stylesheet */
:root {
    /* These variables are injected dynamically by the application */
}

/* Modern Button Styles */
button {
    outline: none;
    background: var(--button-normal-background);
    color: var(--button-normal-text);
    font-family: "Mononoki Nerd Font", monospace;
    font-size: 48px;
    border: none;
    padding: 12px 24px;
    border-radius: 10px;
    transition: background-color 0.7s ease, box-shadow 0.5s ease, transform 0.3s ease;
    transform: scale(1);
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
}

button:hover,
button:active:hover {
    background: var(--button-hover-background);
    box-shadow: 0 8px 16px rgba(0, 0, 0, 0.3);
    transform: translateY(-2px) scale(1.02);
}

button:focus {
    background: var(--button-focus-background);
    box-shadow: 0 6px 12px rgba(0, 0, 0, 0.25);
}

button:active,
button:active:focus,
button:focus:active {
    background: var(--palette12);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.25); /* Smaller shadow for pressed state */
    transform: scale(0.95);
}


/* Specific active state for Lock button */
button#lock-button:active > label,
button#lock-button:active:focus > label,
button#lock-button:focus:active > label {
    color: var(--palette7);
}

/* Specific active state for Logout button */
button#logout-button:active > label,
button#logout-button:active:focus > label,
button#logout-button:focus:active > label {
    color: var(--palette8);
}

/* Specific active state for Reboot button */
button#reboot-button:active > label,
button#reboot-button:active:focus > label,
button#reboot-button:focus:active > label {
    color: var(--palette9);
}

/* Specific active state for Poweroff button */
button#poweroff-button:active > label,
button#poweroff-button:active:focus > label,
button#poweroff-button:focus:active > label {
    color: var(--palette6);
}


.popup-text {
    background: var(--background);
    color: var(--foreground);
    padding: 6px 10px;
    border: 1px solid var(--palette4); /* using a subtle tone */
    border-radius: 6px;
    font-size: 16px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.15);
    transition: opacity 0.3s ease, transform 0.3s ease;
    opacity: 0; /* hidden by default */
    transform: translateY(5px); /* slight slide effect */
}

/* Example: show popup when button is hovered or focused */
button:hover .popup-text,
button:focus .popup-text {
    opacity: 1;
    transform: translateY(0);
}

/* Modern Window Styles */
window {
    background: var(--background);
    border: none;
    padding: 8px;
    border-radius: 12px;
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.15);
    transition: box-shadow 0.3s ease, transform 0.3s ease;
}

window:hover {
    box-shadow: 0 6px 12px rgba(0, 0, 0, 0.2);
    transform: translateY(-3px);
}

/* Grid Styles */
grid {
    padding: 4px;
    transition: background 0.3s ease;
}

/* Animations */
body {
    animation: fadeIn 2s ease-in forwards;
    opacity: 0;
}

@keyframes fadeIn {
    from {
        opacity: 0;
    }
    to {
        opacity: 1;
    }
}

@keyframes fadeOut {
    from {
        opacity: 1;
    }
    to {
        opacity: 0;
    }
}

body.fade-out {
    animation: fadeOut 2s ease-out forwards;
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