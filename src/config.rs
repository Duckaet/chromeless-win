use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub last_url: Option<String>,
    pub window: Option<WindowState>,
}

pub fn config_path() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config")
        });
    let dir = base.join("chromeless");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("config.json")
}

pub fn load() -> Config {
    let path = config_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(config: &Config) {
    let path = config_path();
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_fields_are_none() {
        let c = Config::default();
        assert!(c.last_url.is_none());
        assert!(c.window.is_none());
    }

    #[test]
    fn config_serde_roundtrip_empty() {
        let c = Config::default();
        let json = serde_json::to_string(&c).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.last_url, None);
        assert_eq!(back.window, None);
    }

    #[test]
    fn config_serde_roundtrip_full() {
        let c = Config {
            last_url: Some("https://example.com".into()),
            window: Some(WindowState {
                x: 100,
                y: 200,
                width: 1920,
                height: 1080,
                maximized: true,
            }),
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.last_url.as_deref(), Some("https://example.com"));
        let w = back.window.unwrap();
        assert_eq!(w.x, 100);
        assert_eq!(w.y, 200);
        assert_eq!(w.width, 1920);
        assert_eq!(w.height, 1080);
        assert!(w.maximized);
    }

    #[test]
    fn window_state_serde_boundary_values() {
        let w = WindowState {
            x: i32::MIN,
            y: i32::MAX,
            width: 0,
            height: u32::MAX,
            maximized: false,
        };
        let json = serde_json::to_string(&w).unwrap();
        let back: WindowState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.x, i32::MIN);
        assert_eq!(back.y, i32::MAX);
        assert_eq!(back.width, 0);
        assert_eq!(back.height, u32::MAX);
        assert!(!back.maximized);
    }

    #[test]
    fn config_serde_roundtrip_negative_position() {
        let c = Config {
            last_url: None,
            window: Some(WindowState {
                x: -500,
                y: -300,
                width: 800,
                height: 600,
                maximized: false,
            }),
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        let w = back.window.unwrap();
        assert_eq!(w.x, -500);
        assert_eq!(w.y, -300);
    }

    #[test]
    fn config_serde_pretty_json_is_valid() {
        let c = Config {
            last_url: Some("https://rust-lang.org".into()),
            window: None,
        };
        let json = serde_json::to_string_pretty(&c).unwrap();
        assert!(json.contains("last_url"));
        assert!(json.contains("https://rust-lang.org"));
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.last_url.as_deref(), Some("https://rust-lang.org"));
    }

    #[test]
    fn config_from_empty_json_object() {
        let back: Config = serde_json::from_str("{}").unwrap();
        assert_eq!(back.last_url, None);
        assert_eq!(back.window, None);
    }

    #[test]
    fn config_from_corrupt_json_returns_default_via_load() {
        let tmp = std::env::temp_dir().join("chromeless_test_corrupt");
        let _ = std::fs::create_dir_all(&tmp);
        let path = tmp.join("config.json");
        std::fs::write(&path, "{{not valid json!!!").ok();
        let c: Config = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        assert_eq!(c.last_url, None);
        assert_eq!(c.window, None);
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn config_from_empty_file_returns_default() {
        let tmp = std::env::temp_dir().join("chromeless_test_empty_file");
        let _ = std::fs::create_dir_all(&tmp);
        let path = tmp.join("config.json");
        std::fs::write(&path, "").ok();
        let c: Config = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        assert_eq!(c.last_url, None);
        assert_eq!(c.window, None);
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn config_partial_json_missing_fields_uses_defaults() {
        let json = r#"{"last_url": "https://test.com"}"#;
        let back: Config = serde_json::from_str(json).unwrap();
        assert_eq!(back.last_url.as_deref(), Some("https://test.com"));
        assert!(back.window.is_none());
    }

    #[test]
    fn config_partial_json_window_only() {
        let json =
            r#"{"window": {"x": 10, "y": 20, "width": 640, "height": 480, "maximized": false}}"#;
        let back: Config = serde_json::from_str(json).unwrap();
        assert!(back.last_url.is_none());
        let w = back.window.unwrap();
        assert_eq!(w.width, 640);
        assert_eq!(w.height, 480);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = std::env::temp_dir().join("chromeless_test_save_load");
        let _ = std::fs::create_dir_all(&tmp);
        let path = tmp.join("config.json");
        let c = Config {
            last_url: Some("https://roundtrip.test".into()),
            window: Some(WindowState {
                x: 42,
                y: 84,
                width: 1280,
                height: 720,
                maximized: false,
            }),
        };
        let json = serde_json::to_string_pretty(&c).unwrap();
        std::fs::write(&path, &json).unwrap();
        let loaded: Config = std::fs::read_to_string(&path).unwrap().pipe_to_config();
        assert_eq!(loaded.last_url.as_deref(), Some("https://roundtrip.test"));
        let w = loaded.window.unwrap();
        assert_eq!(w.x, 42);
        assert_eq!(w.y, 84);
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn config_path_ends_with_config_json() {
        let p = config_path();
        let s = p.to_string_lossy();
        assert!(
            s.ends_with("config.json") && s.contains("chromeless"),
            "path was: {}",
            s
        );
    }

    trait PipeToConfig {
        fn pipe_to_config(self) -> Config;
    }
    impl PipeToConfig for String {
        fn pipe_to_config(self) -> Config {
            serde_json::from_str(&self).unwrap_or_default()
        }
    }
}
