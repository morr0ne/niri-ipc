use anyhow::Result;
use niri_ipc::socket::Socket;
use niri_ipc::{Action, Event, Request, Response, WorkspaceReferenceArg};
use regex::Regex;
use tracing::{debug, info};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let mut events_socket = Socket::connect()?;
    let mut requests_socket = Socket::connect()?;

    let mut pip_window = None;

    let title_regex = Regex::new(r"^Picture-in-Picture$")?;
    let app_id_regex = Regex::new(r"firefox$")?;

    if matches!(
        events_socket.send(Request::EventStream)?,
        Ok(Response::Handled)
    ) {
        info!("Trying to fetch existing windows...");
        if let Ok(Response::Windows(windows)) = requests_socket.send(Request::Windows)? {
            for window in windows {
                let app_id_matches = if let Some(app_id) = window.app_id {
                    app_id_regex.is_match(&app_id)
                } else {
                    true
                };

                if let Some(ref title) = window.title
                    && title_regex.is_match(title)
                    && app_id_matches
                {
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
                        info!(
                            "Workspace {} focused. Moving window {} there...",
                            id, window
                        );

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
                    if let Some(ref title) = window.title
                        && title_regex.is_match(title)
                    {
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
