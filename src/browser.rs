use tao::dpi::PhysicalSize;
use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use tao::window::{Window, WindowBuilder, WindowId};
use wry::WebView;
use wry::WebViewBuilder;
#[cfg(target_os = "windows")]
use wry::WebViewBuilderExtWindows;
use wry::http::Request;

use crate::config::WindowState;
use crate::snap;
use crate::toast;

const DEFAULT_WIDTH: u32 = 1160;
const DEFAULT_HEIGHT: u32 = 760;

pub enum AppEvent {
    DragWindow(WindowId),
    Navigate(WindowId, String),
    PageLoaded(WindowId, String),
    CloseWindow(WindowId),
    NewWindow,
    SnapshotDone(WindowId, Vec<u8>),
    Shortcut(WindowId, String),
}

const INIT_SCRIPT: &str = r#"
(function(){
document.addEventListener('mousedown',function(e){
if(e.ctrlKey&&e.button===0){e.preventDefault();window.ipc.postMessage('drag_window')}
},true);
document.addEventListener('DOMContentLoaded',function(){
window.ipc.postMessage('page:loaded:'+(document.location.href||''))
});
document.addEventListener('keydown',function(e){
if(e.defaultPrevented)return;
var c=e.ctrlKey||e.metaKey,s=e.shiftKey,a=e.altKey;
if(c&&!s&&!a){switch(e.key.toLowerCase()){
case'l':e.preventDefault();window.ipc.postMessage('shortcut:hud');return;
case'r':e.preventDefault();window.ipc.postMessage('shortcut:reload');return;
case'p':e.preventDefault();window.ipc.postMessage('shortcut:pin');return;
case'[':e.preventDefault();window.ipc.postMessage('shortcut:back');return;
case']':e.preventDefault();window.ipc.postMessage('shortcut:forward');return;
case'=':case'+':e.preventDefault();window.ipc.postMessage('shortcut:zoom_in');return;
case'-':e.preventDefault();window.ipc.postMessage('shortcut:zoom_out');return;
case'0':e.preventDefault();window.ipc.postMessage('shortcut:zoom_reset');return;
}}
if(c&&s&&!a){switch(e.key.toLowerCase()){
case'r':e.preventDefault();window.ipc.postMessage('shortcut:hard_reload');return;
case's':e.preventDefault();window.ipc.postMessage('shortcut:screenshot');return;
case'c':e.preventDefault();window.ipc.postMessage('shortcut:copy_url');return;
}}
if(a&&!c&&!s){switch(e.key){
case'ArrowLeft':e.preventDefault();window.ipc.postMessage('shortcut:back');return;
case'ArrowRight':e.preventDefault();window.ipc.postMessage('shortcut:forward');return;
}}
if(!c&&!s&&!a&&e.key==='F11'){e.preventDefault();window.ipc.postMessage('shortcut:fullscreen');return;}
if(!c&&!s&&!a&&e.key==='Escape'){e.preventDefault();window.ipc.postMessage('shortcut:escape');return;}
});
window.showHUD=function(v){
var h=document.getElementById('chromeless-hud');
if(h){var i=document.getElementById('chud-input');if(i){i.value=v||'';i.focus();i.select()}return}
var s=document.createElement('style');
s.id='chud-style';
s.textContent='#chromeless-hud{position:fixed;top:84px;left:50%;transform:translateX(-50%);width:min(620px,calc(100vw-48px));height:52px;background:rgba(28,28,32,0.94);-webkit-backdrop-filter:blur(20px)saturate(1.4);backdrop-filter:blur(20px)saturate(1.4);border:1px solid rgba(255,255,255,0.1);border-radius:26px;z-index:2147483647;display:flex;align-items:center;padding:0 20px;box-shadow:0 8px 40px rgba(0,0,0,0.5);animation:chudIn .15s ease-out;box-sizing:border-box}#chromeless-hud input{width:100%;background:none;border:none;outline:none;color:#e8e8ee;font:16px/1.4 -apple-system,BlinkMacSystemFont,sans-serif;caret-color:#888}#chromeless-hud input::placeholder{color:#555}@keyframes chudIn{from{opacity:0;transform:translateX(-50%)translateY(-8px)}to{opacity:1;transform:translateX(-50%)translateY(0)}}@keyframes chudOut{from{opacity:1}to{opacity:0;transform:translateX(-50%)translateY(-8px)}}';
document.head.appendChild(s);
var h2=document.createElement('div');
h2.id='chromeless-hud';
h2.innerHTML='<input type="text" id="chud-input" spellcheck="false" autofocus>';
document.body.appendChild(h2);
var i2=document.getElementById('chud-input');
if(v)i2.value=v;
i2.addEventListener('keydown',function(e){
if(e.key==='Enter'){e.preventDefault();var val=this.value;hideHUD();window.ipc.postMessage('hud:commit:'+val)}
else if(e.key==='Escape'){e.preventDefault();hideHUD()}
});
i2.focus();i2.select();
};
window.hideHUD=function(){
var h=document.getElementById('chromeless-hud');
if(h){h.style.animation='chudOut .12s ease-in forwards';setTimeout(function(){h.remove()},120)}
var s=document.getElementById('chud-style');if(s)s.remove()
};
window.showToast=function(m){
var e=document.getElementById('chromeless-toast');
if(e)e.remove();
var t=document.createElement('div');
t.id='chromeless-toast';
t.textContent=m;
t.style.cssText='position:fixed;bottom:28px;left:50%;transform:translateX(-50%);padding:8px 20px;background:rgba(28,28,32,0.92);-webkit-backdrop-filter:blur(16px)saturate(1.4);backdrop-filter:blur(16px)saturate(1.4);color:#e8e8ee;border-radius:17px;font:13px/1.4 -apple-system,BlinkMacSystemFont,sans-serif;font-weight:500;z-index:2147483647;pointer-events:none;box-shadow:0 4px 20px rgba(0,0,0,0.3);transition:opacity .4s ease-out;opacity:1;border:1px solid rgba(255,255,255,0.06);white-space:nowrap';
document.body.appendChild(t);
setTimeout(function(){t.style.opacity='0';setTimeout(function(){if(t.parentNode)t.remove()},400)},1300);
};
})();
"#;

pub struct BrowserWindow {
    pub window: Window,
    pub webview: WebView,
    pub id: WindowId,
    pub on_start_page: bool,
    pub is_pinned: bool,
    pub zoom_level: f64,
    pub current_url: Option<String>,
}

impl BrowserWindow {
    pub fn new(
        elwt: &EventLoopWindowTarget<AppEvent>,
        proxy: EventLoopProxy<AppEvent>,
        url: Option<&str>,
        window_state: Option<&WindowState>,
    ) -> Self {
        let size = window_state
            .map(|ws| PhysicalSize::new(ws.width.max(320), ws.height.max(220)))
            .unwrap_or(PhysicalSize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT));

        let window = WindowBuilder::new()
            .with_decorations(false)
            .with_inner_size(size)
            .with_title("chromeless")
            .with_min_inner_size(PhysicalSize::new(320, 220))
            .with_visible(false)
            .build(elwt)
            .unwrap_or_else(|e| panic!("chromeless: failed to create window: {}", e));

        if let Some(ws) = window_state {
            window.set_outer_position(tao::dpi::PhysicalPosition::new(ws.x, ws.y));
            if ws.maximized {
                window.set_maximized(true);
            }
        }

        let window_id = window.id();

        let handler = {
            let proxy = proxy.clone();
            move |req: Request<String>| {
                let body = req.body().clone();
                if body == "drag_window" {
                    let _ = proxy.send_event(AppEvent::DragWindow(window_id));
                } else if let Some(url) = body.strip_prefix("hud:commit:") {
                    let _ = proxy.send_event(AppEvent::Navigate(window_id, url.to_string()));
                } else if let Some(url) = body.strip_prefix("page:loaded:") {
                    let _ = proxy.send_event(AppEvent::PageLoaded(window_id, url.to_string()));
                } else if let Some(name) = body.strip_prefix("shortcut:") {
                    let _ = proxy.send_event(AppEvent::Shortcut(window_id, name.to_string()));
                }
            }
        };

        let mut builder = WebViewBuilder::new()
            .with_initialization_script(INIT_SCRIPT)
            .with_ipc_handler(handler)
            .with_browser_accelerator_keys(false)
            .with_default_context_menus(false);

        if let Some(u) = url {
            builder = builder.with_url(u.to_string());
        } else {
            builder = builder.with_html(start_page_html());
        }

        let webview = builder.build(&window).unwrap_or_else(|e| {
            eprintln!("chromeless: failed to create WebView: {}", e);
            eprintln!("chromeless: on Windows 10, install WebView2 Runtime from:");
            eprintln!("  https://developer.microsoft.com/en-us/microsoft-edge/webview2/");
            std::process::exit(1);
        });

        #[cfg(target_os = "windows")]
        crate::win32::set_rounded_corners(&window);

        window.set_visible(true);

        Self {
            window,
            webview,
            id: window_id,
            on_start_page: url.is_none(),
            is_pinned: false,
            zoom_level: 1.0,
            current_url: url.map(|s| s.to_string()),
        }
    }

    pub fn navigate_to(&self, url: &str) {
        let _ = self.webview.load_url(url);
    }

    pub fn load_start_page(&self) {
        let html = crate::browser::start_page_html();
        let _ = self.webview.load_html(&html);
    }

    pub fn go_back(&self) {
        let _ = self.webview.evaluate_script("history.back()");
    }

    pub fn go_forward(&self) {
        let _ = self.webview.evaluate_script("history.forward()");
    }

    pub fn reload(&self) {
        let _ = self.webview.reload();
    }

    pub fn hard_reload(&self) {
        let _ = self.webview.evaluate_script("location.reload(true)");
    }

    pub fn zoom_in(&mut self) {
        self.zoom_level = (self.zoom_level * 1.1).min(5.0);
        let _ = self.webview.zoom(self.zoom_level);
    }

    pub fn zoom_out(&mut self) {
        self.zoom_level = (self.zoom_level / 1.1).max(0.25);
        let _ = self.webview.zoom(self.zoom_level);
    }

    pub fn reset_zoom(&mut self) {
        self.zoom_level = 1.0;
        let _ = self.webview.zoom(1.0);
    }

    pub fn toggle_pin(&mut self) {
        self.is_pinned = !self.is_pinned;
        self.window.set_always_on_top(self.is_pinned);
    }

    pub fn show_hud(&self) {
        let url = self.current_url.as_deref().unwrap_or("");
        let js = format!("showHUD({:?})", url);
        let _ = self.webview.evaluate_script(&js);
    }

    pub fn hide_hud(&self) {
        let _ = self.webview.evaluate_script("hideHUD()");
    }

    pub fn show_toast(&self, msg: &str) {
        let js = toast::show_toast_js(msg);
        let _ = self.webview.evaluate_script(&js);
    }

    pub fn copy_url(&self) {
        if let Some(ref url) = self.current_url {
            let js = format!("navigator.clipboard.writeText({:?})", url);
            let _ = self.webview.evaluate_script(&js);
        }
    }

    pub fn capture_screenshot(&self) -> Option<Vec<u8>> {
        snap::capture_window_png(&self.window)
    }

    pub fn save_position(&self) -> Option<WindowState> {
        let pos = self.window.outer_position().ok()?;
        let size = self.window.inner_size();
        Some(WindowState {
            x: pos.x,
            y: pos.y,
            width: size.width,
            height: size.height,
            maximized: self.window.is_maximized(),
        })
    }
}

pub fn start_page_html() -> String {
    include_str!("../assets/start.html").to_string()
}
