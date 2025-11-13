//! WCAG AA compliance tests for theme color contrast ratios
//!
//! Validates that the theme meets accessibility standards:
//! - Text/panel: ≥4.5:1 (WCAG AA for normal text)
//! - Accent/panel: ≥3.0:1 (WCAG AA for UI components)
//! - JSON syntax: ≥7.0:1 (WCAG AAA for better readability)

use nearx::theme::{Rgb, Theme};

/// Calculate relative luminance for sRGB color (WCAG formula)
fn relative_luminance(Rgb(r, g, b): Rgb) -> f64 {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;

    let r = if r <= 0.03928 {
        r / 12.92
    } else {
        ((r + 0.055) / 1.055).powf(2.4)
    };
    let g = if g <= 0.03928 {
        g / 12.92
    } else {
        ((g + 0.055) / 1.055).powf(2.4)
    };
    let b = if b <= 0.03928 {
        b / 12.92
    } else {
        ((b + 0.055) / 1.055).powf(2.4)
    };

    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Calculate contrast ratio between two colors (WCAG formula)
fn contrast_ratio(fg: Rgb, bg: Rgb) -> f64 {
    let l1 = relative_luminance(fg);
    let l2 = relative_luminance(bg);

    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

#[test]
fn text_on_panel_meets_wcag_aa() {
    let theme = Theme::default();

    let contrast = contrast_ratio(theme.text, theme.panel);
    assert!(
        contrast >= 4.5,
        "Text/panel contrast {:.2}:1 fails WCAG AA (need ≥4.5:1)",
        contrast
    );

    println!("✓ Text/panel contrast: {:.2}:1 (WCAG AA)", contrast);
}

#[test]
fn text_dim_on_panel_meets_wcag_aa() {
    let theme = Theme::default();

    let contrast = contrast_ratio(theme.text_dim, theme.panel);
    assert!(
        contrast >= 4.5,
        "Text dim/panel contrast {:.2}:1 fails WCAG AA (need ≥4.5:1)",
        contrast
    );

    println!("✓ Text dim/panel contrast: {:.2}:1 (WCAG AA)", contrast);
}

#[test]
fn accent_on_panel_meets_wcag_aa_ui() {
    let theme = Theme::default();

    let contrast = contrast_ratio(theme.accent, theme.panel);
    assert!(
        contrast >= 3.0,
        "Accent/panel contrast {:.2}:1 fails WCAG AA for UI components (need ≥3.0:1)",
        contrast
    );

    println!("✓ Accent/panel contrast: {:.2}:1 (WCAG AA UI)", contrast);
}

#[test]
fn accent_strong_on_panel_meets_wcag_aa_ui() {
    let theme = Theme::default();

    let contrast = contrast_ratio(theme.accent_strong, theme.panel);
    assert!(
        contrast >= 3.0,
        "Accent strong/panel contrast {:.2}:1 fails WCAG AA for UI components (need ≥3.0:1)",
        contrast
    );

    println!(
        "✓ Accent strong/panel contrast: {:.2}:1 (WCAG AA UI)",
        contrast
    );
}

#[test]
fn json_syntax_meets_wcag_aaa() {
    let theme = Theme::default();

    let json_colors = [
        ("JSON key", theme.json_key),
        ("JSON string", theme.json_string),
        ("JSON number", theme.json_number),
        ("JSON bool", theme.json_bool),
        ("JSON struct", theme.json_struct),
    ];

    for (name, color) in &json_colors {
        let contrast = contrast_ratio(*color, theme.json_bg);
        assert!(
            contrast >= 7.0,
            "{} contrast {:.2}:1 fails WCAG AAA (need ≥7.0:1)",
            name,
            contrast
        );

        println!("✓ {}: {:.2}:1 (WCAG AAA)", name, contrast);
    }
}

#[test]
fn all_contrasts_meet_requirements() {
    let theme = Theme::default();

    // Text contrasts (WCAG AA - 4.5:1)
    assert!(contrast_ratio(theme.text, theme.panel) >= 4.5);
    assert!(contrast_ratio(theme.text_dim, theme.panel) >= 4.5);

    // UI component contrasts (WCAG AA - 3.0:1)
    assert!(contrast_ratio(theme.accent, theme.panel) >= 3.0);
    assert!(contrast_ratio(theme.accent_strong, theme.panel) >= 3.0);
    assert!(contrast_ratio(theme.success, theme.panel) >= 3.0);
    assert!(contrast_ratio(theme.warn, theme.panel) >= 3.0);
    assert!(contrast_ratio(theme.error, theme.panel) >= 3.0);

    // JSON syntax (WCAG AAA - 7.0:1)
    assert!(contrast_ratio(theme.json_key, theme.json_bg) >= 7.0);
    assert!(contrast_ratio(theme.json_string, theme.json_bg) >= 7.0);
    assert!(contrast_ratio(theme.json_number, theme.json_bg) >= 7.0);
    assert!(contrast_ratio(theme.json_bool, theme.json_bg) >= 7.0);
    assert!(contrast_ratio(theme.json_struct, theme.json_bg) >= 7.0);

    println!("✓ All contrast ratios meet or exceed requirements");
}
