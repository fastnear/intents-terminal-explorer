//! UI flags tests - runtime feature toggles for Web/Tauri enhancements

use nearx::flags::UiFlags;

#[test]
fn default_flags_are_production_ready() {
    let flags = UiFlags::default();

    // Keyboard flags always enabled (both wasm32 and native)
    assert!(
        flags.consume_tab,
        "consume_tab should be enabled by default"
    );
    assert!(flags.dpr_snap, "dpr_snap should be enabled by default");

    // Mouse flags are platform-specific:
    // - wasm32 (Web/Tauri): enabled
    // - native (TUI): disabled
    #[cfg(target_arch = "wasm32")]
    {
        assert!(flags.mouse_map, "mouse_map should be enabled on wasm32");
        assert!(
            flags.dblclick_details,
            "dblclick_details should be enabled on wasm32"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        assert!(
            !flags.mouse_map,
            "mouse_map should be disabled on native (TUI)"
        );
        assert!(
            !flags.dblclick_details,
            "dblclick_details should be disabled on native (TUI)"
        );
    }
}

#[test]
fn all_disabled_disables_everything() {
    let flags = UiFlags::all_disabled();

    assert!(!flags.consume_tab);
    assert!(!flags.dpr_snap);
    assert!(!flags.mouse_map);
    assert!(!flags.dblclick_details);
}

#[test]
fn keyboard_only_enables_keyboard_features() {
    let flags = UiFlags::keyboard_only();

    // Keyboard features enabled
    assert!(flags.consume_tab);

    // Visual/mouse features disabled
    assert!(!flags.dpr_snap);
    assert!(!flags.mouse_map);
    assert!(!flags.dblclick_details);
}

#[test]
fn flags_are_copyable() {
    let flags1 = UiFlags::default();
    let flags2 = flags1; // Should be Copy

    // Both should be independent
    assert_eq!(flags1.consume_tab, flags2.consume_tab);
    assert_eq!(flags1.dpr_snap, flags2.dpr_snap);
}

#[test]
fn flags_can_be_customized() {
    // Customize specific features using struct initialization
    let flags = UiFlags {
        consume_tab: false,
        mouse_map: false,
        dpr_snap: true,
        dblclick_details: true,
    };

    assert!(!flags.consume_tab);
    assert!(flags.dpr_snap); // Enabled
    assert!(!flags.mouse_map);
    assert!(flags.dblclick_details); // Enabled
}
