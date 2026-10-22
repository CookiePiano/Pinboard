//! Choose the GTK backend before Tauri or any worker thread starts.
//!
//! X11/XWayland supports note positioning, stacking and global shortcuts.
//! Keep an explicit user choice, and allow pure Wayland when X11 is absent.

pub fn configure() {
    if prefer_x11(
        std::env::var_os("GDK_BACKEND").is_some(),
        std::env::var_os("DISPLAY").is_some_and(|value| !value.is_empty()),
    ) {
        std::env::set_var("GDK_BACKEND", "x11");
    }
}

fn prefer_x11(backend_is_set: bool, x11_is_available: bool) -> bool {
    !backend_is_set && x11_is_available
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_xwayland_only_when_available_and_not_overridden() {
        assert!(prefer_x11(false, true));
        assert!(!prefer_x11(true, true));
        assert!(!prefer_x11(false, false));
        assert!(!prefer_x11(true, false));
    }
}
