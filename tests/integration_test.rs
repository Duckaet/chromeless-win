use chromeless_lib::config::{Config, WindowState};
use chromeless_lib::toast::show_toast_js;

#[test]
fn start_page_html_is_nonempty() {
    let html = chromeless_lib::browser::start_page_html();
    assert!(!html.is_empty());
}

#[test]
fn start_page_html_has_title() {
    let html = chromeless_lib::browser::start_page_html();
    assert!(html.contains("<title>chromeless</title>"));
}

#[test]
fn start_page_html_has_doctype() {
    let html = chromeless_lib::browser::start_page_html();
    assert!(html.contains("<!doctype html>"));
}

#[test]
fn start_page_html_has_body() {
    let html = chromeless_lib::browser::start_page_html();
    assert!(html.contains("<body>"));
    assert!(html.contains("</body>"));
}

#[test]
fn start_page_html_mentions_all_shortcuts() {
    let html = chromeless_lib::browser::start_page_html();
    assert!(html.contains("Ctrl L"));
    assert!(html.contains("Ctrl drag"));
    assert!(html.contains("F11"));
    assert!(html.contains("Ctrl Shift S"));
    assert!(html.contains("Ctrl P"));
    assert!(html.contains("Ctrl ["));
    assert!(html.contains("Ctrl ]"));
    assert!(html.contains("esc"));
    assert!(html.contains("Ctrl ="));
    assert!(html.contains("Ctrl"));
    assert!(html.contains("Ctrl 0"));
    assert!(html.contains("Ctrl Shift C"));
}

#[test]
fn start_page_html_no_focus_outlines() {
    let html = chromeless_lib::browser::start_page_html();
    assert!(html.contains("outline: none"));
}

#[test]
fn start_page_html_no_selection_highlight() {
    let html = chromeless_lib::browser::start_page_html();
    assert!(html.contains("::selection"));
}

#[test]
fn toast_js_valid_for_start_page() {
    let html = chromeless_lib::browser::start_page_html();
    assert!(html.contains("</html>"));
    assert!(!html.contains("show_toast_js"));
}

#[test]
fn config_default_matches_deserialized_empty() {
    let from_default = Config::default();
    let from_json: Config = serde_json::from_str("{}").unwrap();
    assert_eq!(from_default.last_url, from_json.last_url);
    assert_eq!(from_default.window.is_some(), from_json.window.is_some());
}

#[test]
fn window_state_serializes_with_all_fields() {
    let w = WindowState {
        x: -100,
        y: 200,
        width: 1920,
        height: 1080,
        maximized: true,
    };
    let json = serde_json::to_string(&w).unwrap();
    assert!(json.contains("\"x\":-100"));
    assert!(json.contains("\"y\":200"));
    assert!(json.contains("\"width\":1920"));
    assert!(json.contains("\"height\":1080"));
    assert!(json.contains("\"maximized\":true"));
}

#[test]
fn toast_output_is_safe_for_html_injection() {
    let js = show_toast_js("<img src=x onerror=alert(1)>");
    assert!(js.contains("t.textContent='<img src=x onerror=alert(1)>'"));
    // Uses textContent (safe, auto-escapes HTML) not innerHTML (unsafe)
    assert!(js.contains("textContent="));
    assert!(!js.contains("innerHTML="));
}

#[test]
fn toast_output_preserves_functional_js_structure() {
    let js = show_toast_js("test");
    assert!(js.starts_with("\n(function(){"));
    assert!(js.ends_with("})();\n"));
}

#[test]
fn config_window_state_zero_dimensions() {
    let w = WindowState {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
        maximized: false,
    };
    let json = serde_json::to_string(&w).unwrap();
    let back: WindowState = serde_json::from_str(&json).unwrap();
    assert_eq!(back.width, 0);
    assert_eq!(back.height, 0);
}

#[test]
fn config_large_window_dimensions() {
    let w = WindowState {
        x: 0,
        y: 0,
        width: 7680,
        height: 4320,
        maximized: false,
    };
    let json = serde_json::to_string(&w).unwrap();
    let back: WindowState = serde_json::from_str(&json).unwrap();
    assert_eq!(back.width, 7680);
    assert_eq!(back.height, 4320);
}

#[test]
fn config_url_with_special_characters() {
    let c = Config {
        last_url: Some("https://example.com/path?q=hello&lang=en#section".into()),
        window: None,
    };
    let json = serde_json::to_string(&c).unwrap();
    let back: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(
        back.last_url.as_deref(),
        Some("https://example.com/path?q=hello&lang=en#section")
    );
}

#[test]
fn config_url_with_unicode() {
    let c = Config {
        last_url: Some("https://example.com/日本語".into()),
        window: None,
    };
    let json = serde_json::to_string(&c).unwrap();
    let back: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(back.last_url.as_deref(), Some("https://example.com/日本語"));
}

#[test]
fn config_url_empty_string() {
    let c = Config {
        last_url: Some("".into()),
        window: None,
    };
    let json = serde_json::to_string(&c).unwrap();
    let back: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(back.last_url.as_deref(), Some(""));
}

#[test]
fn config_url_with_backslashes() {
    let c = Config {
        last_url: Some("C:\\Users\\test\\file.html".into()),
        window: None,
    };
    let json = serde_json::to_string(&c).unwrap();
    let back: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(back.last_url.as_deref(), Some("C:\\Users\\test\\file.html"));
}

#[test]
fn config_json_rejects_invalid_types() {
    let json = r#"{"last_url": 12345}"#;
    let result: Result<Config, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn config_json_rejects_invalid_window_state() {
    let json = r#"{"window": {"x": "not_a_number", "y": 0, "width": 100, "height": 100, "maximized": false}}"#;
    let result: Result<Config, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn toast_special_chars_dont_break_js_syntax() {
    let msgs = vec![
        "hello\\world",
        "it's",
        "line\nbreak",
        "carriage\rreturn",
        "tab\there",
        "quote\"inside",
        "back\\slash'quote",
    ];
    for msg in msgs {
        let js = show_toast_js(msg);
        // Must be valid JS structure
        assert!(js.contains("textContent='"), "failed for: {}", msg);
        assert!(js.contains("})()"), "missing closing for: {}", msg);
    }
}

#[test]
fn config_multiple_roundtrips_preserve_state() {
    let mut c = Config {
        last_url: Some("https://first.com".into()),
        window: Some(WindowState {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
            maximized: false,
        }),
    };

    for i in 0..10 {
        let json = serde_json::to_string(&c).unwrap();
        c = serde_json::from_str(&json).unwrap();
        assert_eq!(
            c.last_url.as_deref(),
            Some("https://first.com"),
            "failed on iteration {}",
            i
        );
        let w = c.window.as_ref().unwrap();
        assert_eq!(w.width, 800);
    }
}
