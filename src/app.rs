use std::collections::HashMap;
use std::time::Instant;

use tao::event::WindowEvent;
use tao::event_loop::{ControlFlow, EventLoopWindowTarget};
use tao::window::WindowId;

use crate::browser::{AppEvent, BrowserWindow};
use crate::config::{self, Config, WindowState};

pub struct SnapJob {
    pub path: String,
    pub wait_secs: f64,
}

#[derive(Default)]
pub struct LaunchOptions {
    pub url: Option<String>,
    pub snap: Option<SnapJob>,
    pub size: Option<(u32, u32)>,
}

pub fn smart_url(input: &str) -> Option<String> {
    let t = input.trim();
    if t.is_empty() {
        return None;
    }
    if t.starts_with('/') || t.starts_with('~') {
        return Some(t.to_string());
    }
    if t.contains("://") {
        return Some(t.to_string());
    }
    let lower = t.to_lowercase();
    for host in &["localhost", "127.0.0.1", "0.0.0.0", "[::1]"] {
        if lower.starts_with(host) {
            return Some(format!("http://{}", t));
        }
    }
    if !t.contains(' ') && t.contains('.') {
        if lower.starts_with("https://") || lower.starts_with("http://") {
            return Some(t.to_string());
        }
        return Some(format!("https://{}", t));
    }
    let q: String = t
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | ' ' => c.to_string(),
            _ => c
                .to_string()
                .bytes()
                .map(|b| format!("%{:02X}", b))
                .collect(),
        })
        .collect::<String>()
        .replace(' ', "+");
    Some(format!("https://www.google.com/search?q={}", q))
}

pub struct App {
    pub windows: HashMap<WindowId, BrowserWindow>,
    pub proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    pub config: Config,
    pub pending_save: bool,
    pub last_save: Instant,
    pub launch_options: LaunchOptions,
    pub snap_timer: Option<Instant>,
}

impl App {
    pub fn new(proxy: tao::event_loop::EventLoopProxy<AppEvent>, options: LaunchOptions) -> Self {
        let config = config::load();
        Self {
            windows: HashMap::new(),
            proxy,
            config,
            pending_save: false,
            last_save: Instant::now(),
            launch_options: options,
            snap_timer: None,
        }
    }

    pub fn create_initial_window(&mut self, elwt: &EventLoopWindowTarget<AppEvent>) {
        let url = self.launch_options.url.as_deref();
        let ws = self.config.window.as_ref();
        let bw = BrowserWindow::new(elwt, self.proxy.clone(), url, ws);
        let id = bw.id;
        self.windows.insert(id, bw);

        if self.launch_options.snap.is_some() {
            self.snap_timer = Some(Instant::now() + std::time::Duration::from_secs(30));
        }
    }

    pub fn new_window(&mut self, elwt: &EventLoopWindowTarget<AppEvent>) {
        let bw = BrowserWindow::new(elwt, self.proxy.clone(), None, None);
        self.windows.insert(bw.id, bw);
    }

    pub fn handle_window_event(&mut self, window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(bw) = self.windows.get(&window_id)
                    && let Some(ws) = bw.save_position()
                {
                    self.config.window = Some(ws);
                    self.pending_save = true;
                }
                self.windows.remove(&window_id);
            }
            WindowEvent::Moved(pos) => {
                if let Some(bw) = self.windows.get(&window_id) {
                    let size = bw.window.inner_size();
                    self.config.window = Some(WindowState {
                        x: pos.x,
                        y: pos.y,
                        width: size.width,
                        height: size.height,
                        maximized: bw.window.is_maximized(),
                    });
                    self.pending_save = true;
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(bw) = self.windows.get(&window_id)
                    && let Ok(pos) = bw.window.outer_position()
                {
                    self.config.window = Some(WindowState {
                        x: pos.x,
                        y: pos.y,
                        width: size.width,
                        height: size.height,
                        maximized: bw.window.is_maximized(),
                    });
                    self.pending_save = true;
                }
            }
            _ => {}
        }
    }

    pub fn handle_user_event(&mut self, event: AppEvent, _control_flow: &mut ControlFlow) {
        match event {
            AppEvent::DragWindow(window_id) => {
                if let Some(bw) = self.windows.get(&window_id) {
                    let _ = bw.window.drag_window();
                }
            }
            AppEvent::Navigate(window_id, input) => {
                if let Some(url) = smart_url(&input)
                    && let Some(bw) = self.windows.get(&window_id)
                {
                    bw.navigate_to(&url);
                }
            }
            AppEvent::PageLoaded(window_id, url) => {
                if let Some(bw) = self.windows.get_mut(&window_id) {
                    bw.on_start_page = url.is_empty() || url == "about:blank";
                    if !bw.on_start_page {
                        bw.current_url = Some(url.clone());
                        self.config.last_url = Some(url.clone());
                        self.pending_save = true;
                    }

                    if self.launch_options.snap.is_some() {
                        let wait = self
                            .launch_options
                            .snap
                            .as_ref()
                            .map(|s| s.wait_secs)
                            .unwrap_or(1.0);
                        self.snap_timer =
                            Some(Instant::now() + std::time::Duration::from_secs_f64(wait));
                    }
                }
            }
            AppEvent::CloseWindow(window_id) => {
                if let Some(bw) = self.windows.get(&window_id)
                    && let Some(ws) = bw.save_position()
                {
                    self.config.window = Some(ws);
                    self.pending_save = true;
                }
                self.windows.remove(&window_id);
            }
            AppEvent::SnapshotDone(window_id, png_data) => {
                let desktop = desktop_path();
                let ts = timestamp();
                let filename = format!("chromeless {}.png", ts);
                let path = std::path::Path::new(&desktop).join(&filename);
                if std::fs::write(&path, &png_data).is_ok() {
                    if let Some(bw) = self.windows.get(&window_id) {
                        bw.show_toast(&format!("Saved \"{}\" to Desktop", filename));
                    }
                } else if let Some(bw) = self.windows.get(&window_id) {
                    bw.show_toast("Snapshot failed");
                }
            }
            AppEvent::NewWindow => {}
            AppEvent::Shortcut(window_id, ref name) => {
                if let Some(bw) = self.windows.get_mut(&window_id) {
                    match name.as_str() {
                        "hud" => bw.show_hud(),
                        "reload" => bw.reload(),
                        "hard_reload" => bw.hard_reload(),
                        "screenshot" => {
                            if let Some(png) = bw.capture_screenshot() {
                                let _ = self
                                    .proxy
                                    .send_event(AppEvent::SnapshotDone(window_id, png));
                            }
                        }
                        "pin" => {
                            bw.toggle_pin();
                            let msg = if bw.is_pinned {
                                "Pinned on top"
                            } else {
                                "Unpinned"
                            };
                            bw.show_toast(msg);
                        }
                        "back" => bw.go_back(),
                        "forward" => bw.go_forward(),
                        "zoom_in" => bw.zoom_in(),
                        "zoom_out" => bw.zoom_out(),
                        "zoom_reset" => bw.reset_zoom(),
                        "copy_url" => bw.copy_url(),
                        "fullscreen" => {
                            let fs = bw.window.fullscreen().is_some();
                            bw.window.set_fullscreen(if fs {
                                None
                            } else {
                                Some(tao::window::Fullscreen::Borderless(None))
                            });
                        }
                        "escape" if !bw.on_start_page => {
                            bw.load_start_page();
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn tick_snap(&mut self, control_flow: &mut ControlFlow) {
        let Some(deadline) = self.snap_timer else {
            return;
        };
        if Instant::now() < deadline {
            return;
        }
        self.snap_timer = None;
        if let Some(snap) = self.launch_options.snap.take() {
            let png = self
                .windows
                .values()
                .next()
                .and_then(|bw| bw.capture_screenshot());
            match png {
                Some(data) => {
                    if std::fs::write(&snap.path, &data).is_ok() {
                        eprintln!("saved {} ({} bytes)", snap.path, data.len());
                        *control_flow = ControlFlow::Exit;
                    } else {
                        eprintln!("chromeless: could not write PNG to {}", snap.path);
                        *control_flow = ControlFlow::ExitWithCode(3);
                    }
                }
                None => {
                    eprintln!("chromeless: snapshot failed");
                    *control_flow = ControlFlow::ExitWithCode(3);
                }
            }
        }
    }

    pub fn try_save_config(&mut self) {
        if self.pending_save && self.last_save.elapsed().as_millis() >= 500 {
            config::save(&self.config);
            self.pending_save = false;
            self.last_save = Instant::now();
        }
    }

    pub fn should_exit(&self) -> bool {
        self.windows.is_empty() && self.launch_options.snap.is_none()
    }
}

fn desktop_path() -> String {
    if let Ok(p) = std::env::var("USERPROFILE") {
        let d = std::path::Path::new(&p).join("Desktop");
        if d.exists() {
            return d.to_string_lossy().to_string();
        }
    }
    std::env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

fn timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = d.as_secs();
    let mut approx_year = 1970i64;
    let mut remaining_secs = total_secs as i64;
    loop {
        let year_secs = if is_leap(approx_year) {
            31622400
        } else {
            31536000
        };
        if remaining_secs < year_secs {
            break;
        }
        remaining_secs -= year_secs;
        approx_year += 1;
    }
    let (month, day) = yday_to_md(remaining_secs / 86400, is_leap(approx_year));
    let time = remaining_secs % 86400;
    let hours = time / 3600;
    let mins = (time % 3600) / 60;
    let secs = time % 60;
    format!(
        "{:04}-{:02}-{:02} at {:02}.{:02}.{:02}",
        approx_year, month, day, hours, mins, secs
    )
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

fn yday_to_md(yday: i64, leap: bool) -> (i64, i64) {
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut remaining = yday;
    for (i, &d) in days.iter().enumerate() {
        if remaining < d {
            return (i as i64 + 1, remaining + 1);
        }
        remaining -= d;
    }
    (12, 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── smart_url tests ──

    #[test]
    fn smart_url_empty_returns_none() {
        assert!(smart_url("").is_none());
    }

    #[test]
    fn smart_url_whitespace_only_returns_none() {
        assert!(smart_url("   ").is_none());
    }

    #[test]
    fn smart_url_tabs_returns_none() {
        assert!(smart_url("\t\n\r").is_none());
    }

    #[test]
    fn smart_url_absolute_path_passthrough() {
        assert_eq!(
            smart_url("/home/user/file.html"),
            Some("/home/user/file.html".into())
        );
    }

    #[test]
    fn smart_url_tilde_path_passthrough() {
        assert_eq!(
            smart_url("~/documents/file.txt"),
            Some("~/documents/file.txt".into())
        );
    }

    #[test]
    fn smart_url_full_http_passthrough() {
        assert_eq!(
            smart_url("http://example.com"),
            Some("http://example.com".into())
        );
    }

    #[test]
    fn smart_url_full_https_passthrough() {
        assert_eq!(
            smart_url("https://example.com"),
            Some("https://example.com".into())
        );
    }

    #[test]
    fn smart_url_ftp_passthrough() {
        assert_eq!(
            smart_url("ftp://files.example.com/file"),
            Some("ftp://files.example.com/file".into())
        );
    }

    #[test]
    fn smart_url_file_protocol_passthrough() {
        assert_eq!(
            smart_url("file:///C:/Users/test.html"),
            Some("file:///C:/Users/test.html".into())
        );
    }

    #[test]
    fn smart_url_localhost_prepends_http() {
        assert_eq!(
            smart_url("localhost:3000"),
            Some("http://localhost:3000".into())
        );
    }

    #[test]
    fn smart_url_localhost_uppercase_prepends_http() {
        assert_eq!(
            smart_url("LOCALHOST:8080"),
            Some("http://LOCALHOST:8080".into())
        );
    }

    #[test]
    fn smart_url_127_0_0_1_prepends_http() {
        assert_eq!(
            smart_url("127.0.0.1:8080"),
            Some("http://127.0.0.1:8080".into())
        );
    }

    #[test]
    fn smart_url_0_0_0_0_prepends_http() {
        assert_eq!(
            smart_url("0.0.0.0:3000"),
            Some("http://0.0.0.0:3000".into())
        );
    }

    #[test]
    fn smart_url_ipv6_loopback_prepends_http() {
        assert_eq!(smart_url("[::1]:8080"), Some("http://[::1]:8080".into()));
    }

    #[test]
    fn smart_url_domain_with_dot_prepends_https() {
        assert_eq!(smart_url("example.com"), Some("https://example.com".into()));
    }

    #[test]
    fn smart_url_subdomain_prepends_https() {
        assert_eq!(
            smart_url("sub.example.com"),
            Some("https://sub.example.com".into())
        );
    }

    #[test]
    fn smart_url_domain_with_port_prepends_https() {
        assert_eq!(
            smart_url("example.com:8080"),
            Some("https://example.com:8080".into())
        );
    }

    #[test]
    fn smart_url_ip_address_prepends_https() {
        assert_eq!(smart_url("192.168.1.1"), Some("https://192.168.1.1".into()));
    }

    #[test]
    fn smart_url_search_query_no_dots() {
        let result = smart_url("rust programming language").unwrap();
        assert!(result.starts_with("https://www.google.com/search?q="));
        assert!(result.contains("rust+programming+language"));
    }

    #[test]
    fn smart_url_search_query_special_chars_encoded() {
        let result = smart_url("hello&world=1").unwrap();
        assert!(result.contains("hello%26world%3D1"));
    }

    #[test]
    fn smart_url_search_query_preserves_alphanumeric() {
        let result = smart_url("test123").unwrap();
        assert!(result.contains("test123"));
        assert!(!result.contains("%"));
    }

    #[test]
    fn smart_url_search_encodes_unicode() {
        let result = smart_url("日本語テスト").unwrap();
        assert!(result.starts_with("https://www.google.com/search?q="));
        assert!(result.contains('%'));
    }

    #[test]
    fn smart_url_leading_trailing_whitespace_trimmed() {
        assert_eq!(
            smart_url("  https://example.com  "),
            Some("https://example.com".into())
        );
    }

    #[test]
    fn smart_url_search_with_mixed_case() {
        let result = smart_url("Hello World").unwrap();
        assert!(result.contains("Hello+World"));
    }

    #[test]
    fn smart_url_search_single_word() {
        let result = smart_url("rustlang").unwrap();
        assert!(result.contains("https://www.google.com/search?q=rustlang"));
    }

    #[test]
    fn smart_url_protocol_with_path() {
        assert_eq!(
            smart_url("https://example.com/path/to/page"),
            Some("https://example.com/path/to/page".into())
        );
    }

    #[test]
    fn smart_url_weird_protocol_passthrough() {
        assert_eq!(
            smart_url("myapp://something"),
            Some("myapp://something".into())
        );
    }

    // ── is_leap tests ──

    #[test]
    fn is_leap_2024() {
        assert!(is_leap(2024));
    }

    #[test]
    fn is_leap_2023() {
        assert!(!is_leap(2023));
    }

    #[test]
    fn is_leap_2000_div_by_400() {
        assert!(is_leap(2000));
    }

    #[test]
    fn is_leap_1900_div_by_100_not_400() {
        assert!(!is_leap(1900));
    }

    #[test]
    fn is_leap_1600_div_by_400() {
        assert!(is_leap(1600));
    }

    #[test]
    fn is_leap_1996_div_by_4_not_100() {
        assert!(is_leap(1996));
    }

    #[test]
    fn is_leap_1998_not_div_by_4() {
        assert!(!is_leap(1998));
    }

    #[test]
    fn is_leap_1970_not_leap() {
        assert!(!is_leap(1970));
    }

    #[test]
    fn is_leap_year_0_div_by_400() {
        assert!(is_leap(0));
    }

    // ── yday_to_md tests ──

    #[test]
    fn yday_to_md_jan_1_non_leap() {
        assert_eq!(yday_to_md(0, false), (1, 1));
    }

    #[test]
    fn yday_to_md_jan_31_non_leap() {
        assert_eq!(yday_to_md(30, false), (1, 31));
    }

    #[test]
    fn yday_to_md_feb_1_non_leap() {
        assert_eq!(yday_to_md(31, false), (2, 1));
    }

    #[test]
    fn yday_to_md_feb_28_non_leap() {
        assert_eq!(yday_to_md(58, false), (2, 28));
    }

    #[test]
    fn yday_to_md_mar_1_non_leap() {
        assert_eq!(yday_to_md(59, false), (3, 1));
    }

    #[test]
    fn yday_to_md_dec_31_non_leap() {
        assert_eq!(yday_to_md(364, false), (12, 31));
    }

    #[test]
    fn yday_to_md_jan_1_leap() {
        assert_eq!(yday_to_md(0, true), (1, 1));
    }

    #[test]
    fn yday_to_md_feb_29_leap() {
        assert_eq!(yday_to_md(59, true), (2, 29));
    }

    #[test]
    fn yday_to_md_mar_1_leap() {
        assert_eq!(yday_to_md(60, true), (3, 1));
    }

    #[test]
    fn yday_to_md_dec_31_leap() {
        assert_eq!(yday_to_md(365, true), (12, 31));
    }

    #[test]
    fn yday_to_md_mid_year_jul_4() {
        // July 4 = 31+28+31+30+31+30+4 = 185 (0-indexed: 184)
        assert_eq!(yday_to_md(184, false), (7, 4));
    }

    #[test]
    fn yday_to_md_out_of_range_falls_through() {
        assert_eq!(yday_to_md(999, false), (12, 31));
    }

    #[test]
    fn yday_to_md_out_of_range_leap() {
        assert_eq!(yday_to_md(400, true), (12, 31));
    }

    // ── LaunchOptions::default tests ──

    #[test]
    fn launch_options_default_all_none() {
        let opts = LaunchOptions::default();
        assert!(opts.url.is_none());
        assert!(opts.snap.is_none());
        assert!(opts.size.is_none());
    }

    #[test]
    fn snap_job_stores_path_and_wait() {
        let job = SnapJob {
            path: "/tmp/test.png".into(),
            wait_secs: 5.0,
        };
        assert_eq!(job.path, "/tmp/test.png");
        assert_eq!(job.wait_secs, 5.0);
    }

    // ── desktop_path tests ──

    #[test]
    fn desktop_path_returns_string() {
        let p = desktop_path();
        assert!(!p.is_empty());
    }

    #[test]
    fn desktop_path_ends_with_desktop_or_is_cwd() {
        let p = desktop_path();
        let is_desktop = p.ends_with("Desktop") || p.ends_with("desktop");
        let is_cwd = p
            == std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy();
        assert!(is_desktop || is_cwd, "unexpected desktop_path: {}", p);
    }

    // ── timestamp tests ──

    #[test]
    fn timestamp_format_is_correct() {
        let ts = timestamp();
        // Format: YYYY-MM-DD at HH.MM.SS
        assert_eq!(ts.len(), 22, "timestamp length wrong: {}", ts);
        assert!(ts.contains(" at "), "missing ' at ': {}", ts);
        assert!(ts.chars().nth(4) == Some('-'), "no dash after year: {}", ts);
        assert!(
            ts.chars().nth(7) == Some('-'),
            "no dash after month: {}",
            ts
        );
        assert!(
            ts.chars().nth(10) == Some(' '),
            "no space after day: {}",
            ts
        );
    }

    #[test]
    fn timestamp_has_valid_numbers() {
        let ts = timestamp();
        let parts: Vec<&str> = ts.split(" at ").collect();
        assert_eq!(parts.len(), 2);
        let date_parts: Vec<&str> = parts[0].split('-').collect();
        assert_eq!(date_parts.len(), 3);
        assert!(date_parts[0].parse::<u32>().is_ok());
        assert!(date_parts[1].parse::<u32>().is_ok());
        assert!(date_parts[2].parse::<u32>().is_ok());
        let time_parts: Vec<&str> = parts[1].split('.').collect();
        assert_eq!(time_parts.len(), 3);
        assert!(time_parts[0].parse::<u32>().is_ok());
        assert!(time_parts[1].parse::<u32>().is_ok());
        assert!(time_parts[2].parse::<u32>().is_ok());
    }

    #[test]
    fn timestamp_year_is_reasonable() {
        let ts = timestamp();
        let year: u32 = ts[..4].parse().unwrap();
        assert!((2024..=2100).contains(&year), "unreasonable year: {}", year);
    }

    #[test]
    fn timestamp_month_in_range() {
        let ts = timestamp();
        let month: u32 = ts[5..7].parse().unwrap();
        assert!((1..=12).contains(&month), "invalid month: {}", month);
    }

    #[test]
    fn timestamp_day_in_range() {
        let ts = timestamp();
        let day: u32 = ts[8..10].parse().unwrap();
        assert!((1..=31).contains(&day), "invalid day: {}", day);
    }

    #[test]
    fn timestamp_hours_in_range() {
        let ts = timestamp();
        // Format: YYYY-MM-DD at HH.MM.SS
        // Indices: 0-3 year, 4 dash, 5-6 month, 7 dash, 8-9 day, 10-13 " at ", 14-15 hours
        let hours: u32 = ts[14..16].parse().unwrap();
        assert!(hours <= 23, "invalid hours: {}", hours);
    }

    #[test]
    fn timestamp_minutes_in_range() {
        let ts = timestamp();
        let mins: u32 = ts[17..19].parse().unwrap();
        assert!(mins <= 59, "invalid minutes: {}", mins);
    }

    #[test]
    fn timestamp_seconds_in_range() {
        let ts = timestamp();
        let secs: u32 = ts[20..22].parse().unwrap();
        assert!(secs <= 59, "invalid seconds: {}", secs);
    }

    #[test]
    fn timestamp_is_deterministic_within_second() {
        let ts1 = timestamp();
        let ts2 = timestamp();
        assert_eq!(ts1.len(), ts2.len());
    }

    // ── Config path tests ──

    #[test]
    fn config_load_missing_file_returns_default() {
        let tmp = std::env::temp_dir().join("chromeless_test_missing_config");
        let path = tmp.join("nonexistent.json");
        let c: Config = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        assert_eq!(c.last_url, None);
        assert_eq!(c.window, None);
    }

    #[test]
    fn config_save_and_load_roundtrip() {
        let tmp = std::env::temp_dir().join("chromeless_test_roundtrip");
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
        let loaded: Config = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        assert_eq!(loaded.last_url.as_deref(), Some("https://roundtrip.test"));
        let w = loaded.window.unwrap();
        assert_eq!(w.x, 42);
        assert_eq!(w.y, 84);
        let _ = std::fs::remove_dir_all(tmp);
    }
}
