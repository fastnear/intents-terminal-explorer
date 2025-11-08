//! Versioned deep link router for NEARx
//!
//! Supports nearx://v1/* URLs that map to the current explorer UI.
//! Routes are intentionally simple and leverage existing filter + pane focus.
//!
//! ## Supported Routes (v1)
//!
//! - `nearx://v1/tx/<hash>` - Focus transactions pane, filter to hash
//! - `nearx://v1/block/<height>` - Focus blocks pane, filter to height
//! - `nearx://v1/account/<id>` - Focus transactions pane, filter to account
//! - `nearx://v1/home` - Clear filter, return to auto-follow
//!
//! ## Robust Parsing
//!
//! The parser handles various URL formats robustly:
//! - Case-insensitive scheme: `NEARX://`, `nearx://`, `NEARx://`
//! - Single-slash variants: `nearx:/v1/...`
//! - Multiple slashes: `nearx:////v1/...`
//! - Query and fragment stripping: `nearx://v1/tx/ABC?utm=1#frag`
//!
//! ## Web Hash Formats
//!
//! For web environments, also accepts:
//! - `#/v1/tx/<hash>` - Direct hash routing
//! - `#/deeplink/<encodeURIComponent(nearx://...)>` - Encoded deep link
//!
//! ## Example
//!
//! ```rust,ignore
//! use nearx::router::{parse, Route, RouteV1};
//!
//! let route = parse("nearx://v1/tx/ABC123").unwrap();
//! match route {
//!     Route::V1(RouteV1::Tx { hash }) => {
//!         println!("Transaction: {}", hash);
//!     }
//!     _ => {}
//! }
//! ```

/// Strip query and fragment from URL path
#[inline]
fn strip_query_frag(s: &str) -> &str {
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'?' || b == b'#' {
            return &s[..i];
        }
    }
    s
}

/// Extract path after nearx:// scheme (case-insensitive, handles variants)
#[inline]
fn after_nearx_scheme(raw: &str) -> Option<&str> {
    // Accept nearx://, NEARX://, nearx:/, nearx:////...
    let s = raw.trim();
    if let Some(pos) = s.find("://") {
        if s[..pos].eq_ignore_ascii_case("nearx") {
            let mut rest = &s[pos + 3..];
            while rest.starts_with('/') {
                rest = &rest[1..];
            }
            return Some(rest);
        }
    } else if let Some(rest) = s.strip_prefix("nearx:") {
        let mut r = rest;
        while r.starts_with('/') {
            r = &r[1..];
        }
        return Some(r);
    }
    None
}

/// V1 route variants
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteV1 {
    /// Transaction details: `nearx://v1/tx/<hash>`
    Tx { hash: String },
    /// Block details: `nearx://v1/block/<height>`
    Block { height: u64 },
    /// Account transactions: `nearx://v1/account/<id>`
    Account { id: String },
    /// Home (clear state): `nearx://v1/home`
    Home,
}

/// Versioned route container
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    /// Version 1 routes
    V1(RouteV1),
}

/// Parse a route from various URL formats
///
/// Accepts:
/// - `nearx://v1/tx/<hash>`
/// - `nearx://v1/block/<height>`
/// - `nearx://v1/account/<id>`
/// - `nearx://v1/home` or `nearx://v1/` or `nearx://v1`
/// - `#/v1/...` (web hash format)
/// - `#/deeplink/<encoded>` (Tauri bridge format)
/// - `/v1/...` (path only)
///
/// Returns `None` for invalid URLs or unsupported versions.
pub fn parse(raw: &str) -> Option<Route> {
    if raw.is_empty() {
        return Some(Route::V1(RouteV1::Home));
    }

    let s = raw.trim();

    // Extract path component from various formats
    let path = if let Some(rest) = after_nearx_scheme(s) {
        // Robust scheme handling (case-insensitive, slash variants)
        rest
    } else if s.starts_with("#/deeplink/") {
        // Encoded deep link: #/deeplink/<encodeURIComponent(nearx://...)>
        // This format is only used in WASM (Tauri->Web bridge)
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(encoded) = s.strip_prefix("#/deeplink/") {
                // Decode using web APIs - handle Result and convert JsString to String
                match js_sys::decode_uri_component(encoded) {
                    Ok(js_string) => {
                        let decoded = String::from(js_string);
                        // Recursively parse the decoded URL
                        return parse(&decoded);
                    }
                    Err(_) => return None,
                }
            }
        }
        // Non-WASM: not supported (native TUI doesn't use hash routing)
        return None;
    } else if let Some(rest) = s.strip_prefix("#/") {
        // Direct hash: #/v1/...
        rest
    } else if let Some(rest) = s.strip_prefix("/") {
        // Path only: /v1/...
        rest
    } else if s.starts_with("v1/") {
        // Already normalized
        s
    } else {
        s
    };

    // Strip query parameters and fragments
    let path = strip_query_frag(path);

    // Parse version and route: "v1/tx/ABC123" or "v1/block/12345" etc.
    let mut segments = path.split('/').filter(|s| !s.is_empty());

    let version = segments.next()?.to_ascii_lowercase();
    if version != "v1" {
        return None; // Unsupported version
    }

    let page = segments.next().unwrap_or("").to_ascii_lowercase();
    match page.as_str() {
        "" | "home" => Some(Route::V1(RouteV1::Home)),
        "tx" => {
            let hash = segments.next()?.to_string();
            if hash.is_empty() {
                None
            } else {
                Some(Route::V1(RouteV1::Tx { hash }))
            }
        }
        "block" => {
            let height_str = segments.next()?;
            let height = height_str.parse::<u64>().ok()?;
            Some(Route::V1(RouteV1::Block { height }))
        }
        "account" => {
            let id = segments.next()?.to_string();
            if id.is_empty() {
                None
            } else {
                Some(Route::V1(RouteV1::Account { id }))
            }
        }
        _ => None, // Unknown route
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tx() {
        let route = parse("nearx://v1/tx/ABC123").unwrap();
        assert_eq!(
            route,
            Route::V1(RouteV1::Tx {
                hash: "ABC123".to_string()
            })
        );

        let route = parse("#/v1/tx/DEF456").unwrap();
        assert_eq!(
            route,
            Route::V1(RouteV1::Tx {
                hash: "DEF456".to_string()
            })
        );

        let route = parse("/v1/tx/GHI789").unwrap();
        assert_eq!(
            route,
            Route::V1(RouteV1::Tx {
                hash: "GHI789".to_string()
            })
        );
    }

    #[test]
    fn test_parse_block() {
        let route = parse("nearx://v1/block/12345").unwrap();
        assert_eq!(route, Route::V1(RouteV1::Block { height: 12345 }));

        let route = parse("#/v1/block/67890").unwrap();
        assert_eq!(route, Route::V1(RouteV1::Block { height: 67890 }));
    }

    #[test]
    fn test_parse_account() {
        let route = parse("nearx://v1/account/alice.near").unwrap();
        assert_eq!(
            route,
            Route::V1(RouteV1::Account {
                id: "alice.near".to_string()
            })
        );

        let route = parse("#/v1/account/bob.near").unwrap();
        assert_eq!(
            route,
            Route::V1(RouteV1::Account {
                id: "bob.near".to_string()
            })
        );
    }

    #[test]
    fn test_parse_home() {
        assert_eq!(parse("nearx://v1/home").unwrap(), Route::V1(RouteV1::Home));
        assert_eq!(parse("nearx://v1/").unwrap(), Route::V1(RouteV1::Home));
        assert_eq!(parse("nearx://v1").unwrap(), Route::V1(RouteV1::Home));
        assert_eq!(parse("#/v1/home").unwrap(), Route::V1(RouteV1::Home));
        assert_eq!(parse("").unwrap(), Route::V1(RouteV1::Home));
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse("nearx://v2/tx/ABC").is_none()); // Wrong version
        assert!(parse("nearx://v1/tx/").is_none()); // Missing hash
        assert!(parse("nearx://v1/block/abc").is_none()); // Invalid height
        assert!(parse("nearx://v1/unknown/test").is_none()); // Unknown route
    }

    #[test]
    fn test_parse_case_insensitive_scheme() {
        // Upper-case scheme
        let r1 = parse("NEARX://v1/tx/XYZ").unwrap();
        match r1 {
            Route::V1(RouteV1::Tx { hash }) => assert_eq!(hash, "XYZ"),
            _ => panic!("Expected Tx route"),
        }

        // Mixed case
        let r2 = parse("NEARx://v1/block/42").unwrap();
        match r2 {
            Route::V1(RouteV1::Block { height }) => assert_eq!(height, 42),
            _ => panic!("Expected Block route"),
        }
    }

    #[test]
    fn test_parse_query_and_fragment() {
        // Query parameter
        let r1 = parse("nearx://v1/tx/XYZ?utm=1").unwrap();
        match r1 {
            Route::V1(RouteV1::Tx { hash }) => assert_eq!(hash, "XYZ"),
            _ => panic!("Expected Tx route"),
        }

        // Fragment
        let r2 = parse("nearx://v1/block/42#frag").unwrap();
        match r2 {
            Route::V1(RouteV1::Block { height }) => assert_eq!(height, 42),
            _ => panic!("Expected Block route"),
        }

        // Both query and fragment
        let r3 = parse("nearx://v1/account/alice.near?ref=ext#test").unwrap();
        match r3 {
            Route::V1(RouteV1::Account { id }) => assert_eq!(id, "alice.near"),
            _ => panic!("Expected Account route"),
        }
    }

    #[test]
    fn test_parse_single_slash_variant() {
        // Single slash after colon
        let r = parse("nearx:/v1/tx/ABC").unwrap();
        match r {
            Route::V1(RouteV1::Tx { hash }) => assert_eq!(hash, "ABC"),
            _ => panic!("Expected Tx route"),
        }
    }

    #[test]
    fn test_parse_multiple_slashes() {
        // Multiple slashes (sometimes happens with URL builders)
        let r = parse("nearx:////v1/block/123").unwrap();
        match r {
            Route::V1(RouteV1::Block { height }) => assert_eq!(height, 123),
            _ => panic!("Expected Block route"),
        }
    }
}
