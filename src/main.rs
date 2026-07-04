use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};

use chromeless_lib::app::{self, App, LaunchOptions, SnapJob};
use chromeless_lib::browser::AppEvent;
use chromeless_lib::config;

fn main() {
    let options = parse_args();

    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let mut app = App::new(proxy, options);
    app.create_initial_window(&event_loop);

    event_loop.run(move |event, elwt, control_flow| {
        *control_flow = if app.snap_timer.is_some() || app.launch_options.snap.is_some() {
            ControlFlow::Poll
        } else {
            ControlFlow::Wait
        };

        match event {
            Event::WindowEvent {
                window_id, event, ..
            } => {
                app.handle_window_event(window_id, event);
                if app.should_exit() {
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::UserEvent(user_event) => {
                match user_event {
                    AppEvent::NewWindow => {
                        app.new_window(elwt);
                    }
                    other => {
                        app.handle_user_event(other, control_flow);
                    }
                }
                if app.should_exit() {
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::MainEventsCleared => {
                app.tick_snap(control_flow);
                app.try_save_config();
            }
            Event::LoopDestroyed => {
                config::save(&app.config);
            }
            _ => {}
        }
    });
}

fn parse_args() -> LaunchOptions {
    let mut options = LaunchOptions::default();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    let mut snap_path: Option<String> = None;
    let mut snap_wait: f64 = 1.0;

    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                eprintln!(
                    "chromeless — the browser that isn't there\n\
                     \n\
                     usage: chromeless [url] [options]\n\
                       --snap <path>    load the page, save a PNG, and quit\n\
                       --size <WxH>     window size (e.g. 1440x900)\n\
                       --wait <secs>    settle time before --snap (default 1.0)\n\
                     \n\
                     examples:\n\
                       chromeless youtube.com\n\
                       chromeless localhost:3000 --snap shot.png --size 1280x800"
                );
                std::process::exit(0);
            }
            "--snap" => {
                i += 1;
                if i < args.len() {
                    snap_path = Some(args[i].clone());
                }
            }
            "--wait" => {
                i += 1;
                if i < args.len() {
                    snap_wait = args[i].parse().unwrap_or(1.0);
                }
            }
            "--size" => {
                i += 1;
                if i < args.len() {
                    let parts: Vec<&str> = args[i].split(&['x', 'X'][..]).collect();
                    if parts.len() == 2
                        && let (Ok(w), Ok(h)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>())
                    {
                        options.size = Some((w, h));
                    }
                }
            }
            s if s.starts_with('-') => {
                eprintln!("chromeless: ignoring unknown option {}", s);
            }
            url => {
                if let Some(u) = app::smart_url(url) {
                    options.url = Some(u);
                }
            }
        }
        i += 1;
    }

    if let Some(path) = snap_path {
        let resolved = if std::path::Path::new(&path).is_absolute() {
            path
        } else {
            let cwd = std::env::current_dir().unwrap_or_default();
            cwd.join(&path).to_string_lossy().to_string()
        };
        options.snap = Some(SnapJob {
            path: resolved,
            wait_secs: snap_wait,
        });
    }

    options
}
