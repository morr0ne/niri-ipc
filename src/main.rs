use anyhow::Result;
use niri_ipc::socket::Socket;
use niri_ipc::{Action, Event, Request, Response, WorkspaceReferenceArg};
use regex::Regex;

fn main() -> Result<()> {
    let mut events_socket = Socket::connect()?;
    let mut requests_socket = Socket::connect()?;

    let mut pip_window = None;

    let title_regex = Regex::new(r"^Picture-in-Picture$")?;
    let app_id_regex = Regex::new(r"firefox$")?;

    if matches!(
        events_socket.send(Request::EventStream)?,
        Ok(Response::Handled)
    ) {
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
                    pip_window = Some(window.id);
                }
            }
        }

        let mut read_event = events_socket.read_events();

        while let Ok(event) = read_event() {
            match event {
                Event::WorkspaceActivated { id, focused } => {
                    if focused && let Some(window) = pip_window {
                        let _ = requests_socket.send(Request::Action(
                            Action::MoveWindowToWorkspace {
                                window_id: Some(window),
                                reference: WorkspaceReferenceArg::Id(id),
                                focus: false,
                            },
                        ))?;
                    }
                }
                Event::WindowOpenedOrChanged { ref window } => {
                    if let Some(ref title) = window.title
                        && title_regex.is_match(title)
                    {
                        pip_window = Some(window.id);
                    }
                }
                _ => (),
            }
        }
    }

    Ok(())
}
