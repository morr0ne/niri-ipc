use std::cell::LazyCell;

use anyhow::Result;
use niri_ipc::socket::Socket;
use niri_ipc::{Action, Event, Request, Response, Window, WorkspaceReferenceArg};
use regex::Regex;
use tracing::info;

const TITLE_REGEX: LazyCell<Regex> =
    LazyCell::new(|| Regex::new(r"^Picture-in-Picture$").expect("Invalid regex"));

const APP_ID_REGEX: LazyCell<Regex> =
    LazyCell::new(|| Regex::new(r"^Picture-in-Picture$").expect("Invalid regex"));

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let mut events_socket = Socket::connect()?;
    let mut requests_socket = Socket::connect()?;

    let mut pip_window = None;

    if matches!(
        events_socket.send(Request::EventStream)?,
        Ok(Response::Handled)
    ) {
        info!("Trying to fetch existing windows...");
        if let Ok(Response::Windows(windows)) = requests_socket.send(Request::Windows)? {
            for window in windows {
                if window_matches(&window) {
                    info!("Found a matching window with id {}", window.id);
                    pip_window = Some(window.id);
                }
            }
        }

        let mut read_event = events_socket.read_events();

        info!("Starting read of events");

        while let Ok(event) = read_event() {
            match event {
                Event::WorkspaceActivated { id, focused } => {
                    if focused && let Some(window) = pip_window {
                        info!("Workspace {} focused. Moving window {}", id, window);

                        let _ = requests_socket.send(Request::Action(
                            Action::MoveWindowToWorkspace {
                                window_id: Some(window),
                                reference: WorkspaceReferenceArg::Id(id),
                                focus: false,
                            },
                        ))?;
                    } else {
                        info!("Workspace {} focused but no window was detected", id);
                    }
                }
                Event::WindowOpenedOrChanged { ref window } => {
                    if window_matches(window) && pip_window != Some(window.id) {
                        info!("Window {} matched regexs", window.id);
                        pip_window = Some(window.id);
                    }
                }
                Event::WindowClosed { id } => {
                    if let Some(window) = pip_window
                        && window == id
                    {
                        info!("Window {} got closed", window);

                        pip_window = None
                    }
                }
                _ => (),
            }
        }
    }

    Ok(())
}

fn window_matches(window: &Window) -> bool {
    let app_id_matches = if let Some(ref app_id) = window.app_id {
        APP_ID_REGEX.is_match(app_id)
    } else {
        true
    };

    if let Some(ref title) = window.title {
        return TITLE_REGEX.is_match(title);
    }

    false
}
