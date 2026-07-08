use gtk4::gdk;

/// A direction for arrow-key navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrow {
    Up,
    Down,
    Left,
    Right,
}

/// A normalized navigation event that does not depend on GTK key codes.
#[derive(Debug, Clone, Copy)]
pub enum KeyEvent {
    Arrow(Arrow),
    Tab(bool),
    Other,
}

/// Maps a raw GTK key to a normalized `KeyEvent`.
#[must_use]
fn key_to_event(key: gdk::Key) -> KeyEvent {
    match key {
        gdk::Key::Up | gdk::Key::KP_Up => KeyEvent::Arrow(Arrow::Up),
        gdk::Key::Down | gdk::Key::KP_Down => KeyEvent::Arrow(Arrow::Down),
        gdk::Key::Left | gdk::Key::KP_Left => KeyEvent::Arrow(Arrow::Left),
        gdk::Key::Right | gdk::Key::KP_Right => KeyEvent::Arrow(Arrow::Right),
        gdk::Key::Tab => KeyEvent::Tab(true),
        gdk::Key::ISO_Left_Tab => KeyEvent::Tab(false),
        _ => KeyEvent::Other,
    }
}

/// Calculate the new index when moving up. Wraps to the bottom in the same column if necessary.
#[must_use]
fn index_up(current: usize, total: usize, columns: usize) -> usize {
    if let Some(new_index) = current.checked_sub(columns) {
        new_index.min(total.saturating_sub(1))
    } else {
        let col = current % columns;
        let num_rows = total.div_ceil(columns);
        let last_row = num_rows - 1;
        let candidate = last_row * columns + col;
        if candidate < total {
            candidate
        } else {
            total - 1
        }
    }
}

/// Calculate the new index when moving down. Wraps to the top in the same column if necessary.
#[must_use]
fn index_down(current: usize, total: usize, columns: usize) -> usize {
    let candidate = current + columns;
    if candidate < total {
        candidate
    } else {
        current % columns
    }
}

/// Calculate the new index when moving left. Wraps from first button to last.
#[must_use]
fn index_left(current: usize, total: usize) -> usize {
    (current + total - 1) % total
}

/// Calculate the new index when moving right. Wraps from last button to first.
#[must_use]
fn index_right(current: usize, total: usize) -> usize {
    (current + 1) % total
}

/// Computes a new focus index from a normalized `KeyEvent`.
#[must_use]
pub fn next_focus_index(current: usize, total: usize, columns: usize, event: KeyEvent) -> usize {
    match event {
        KeyEvent::Arrow(dir) => match dir {
            Arrow::Up => index_up(current, total, columns),
            Arrow::Down => index_down(current, total, columns),
            Arrow::Left => index_left(current, total),
            Arrow::Right => index_right(current, total),
        },
        KeyEvent::Tab(forward) => {
            if forward {
                (current + 1) % total
            } else {
                (current + total - 1) % total
            }
        }
        KeyEvent::Other => current,
    }
}

/// An immutable wrapper holding the currently focused button index.
#[derive(Debug, Clone, Copy)]
pub struct FocusState(usize);

impl FocusState {
    #[must_use]
    pub fn new() -> Self {
        Self(0)
    }

    #[must_use]
    pub fn current(&self) -> usize {
        self.0
    }

    /// Returns the next state for a pressed key (pure; no GTK interaction).
    #[must_use]
    pub fn transition(&self, key: gdk::Key, total: usize, columns: usize) -> Self {
        let new = next_focus_index(self.0, total, columns, key_to_event(key));
        Self(new)
    }
}

impl Default for FocusState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::init_env;
    use gtk4::gdk;

    #[test]
    fn new_index_for_arrow_up() {
        init_env();
        let index = next_focus_index(3, 6, 2, key_to_event(gdk::Key::Up));
        assert_eq!(index, index_up(3, 6, 2));
    }

    #[test]
    fn new_index_for_arrow_down() {
        init_env();
        let index = next_focus_index(1, 6, 2, key_to_event(gdk::Key::Down));
        assert_eq!(index, index_down(1, 6, 2));
    }

    #[test]
    fn new_index_for_arrow_left() {
        init_env();
        let index = next_focus_index(1, 6, 2, key_to_event(gdk::Key::Left));
        assert_eq!(index, index_left(1, 6));
    }

    #[test]
    fn new_index_for_arrow_right() {
        init_env();
        let index = next_focus_index(0, 6, 2, key_to_event(gdk::Key::Right));
        assert_eq!(index, index_right(0, 6));
    }

    #[test]
    fn calculate_new_index_for_tab_forward() {
        init_env();
        let index = next_focus_index(2, 4, 2, KeyEvent::Tab(true));
        assert_eq!(index, 3);
    }

    #[test]
    fn calculate_new_index_for_tab_backward() {
        init_env();
        let index = next_focus_index(0, 4, 2, KeyEvent::Tab(false));
        assert_eq!(index, 3);
    }
}
