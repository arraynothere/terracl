use crate::{
    app::{App, InputMode},
    ui::{editor_view, popup, sidebar, statusbar},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub const COMPACT_WIDTH: u16 = 70;

pub struct Regions {
    pub sidebar: Option<Rect>,
    pub editor: Option<Rect>,
    pub status: Rect,
}

pub fn is_compact(width: u16) -> bool {
    width < COMPACT_WIDTH
}

pub fn regions(size: Rect, show_sidebar: bool) -> Regions {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(size);
    let workspace = rows[0];
    let status = rows[1];

    if !show_sidebar {
        return Regions {
            sidebar: None,
            editor: Some(workspace),
            status,
        };
    }

    if is_compact(size.width) {
        return Regions {
            sidebar: Some(workspace),
            editor: None,
            status,
        };
    }

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(workspace);

    Regions {
        sidebar: Some(columns[0]),
        editor: Some(columns[1]),
        status,
    }
}

pub fn render(frame: &mut Frame, app: &App) {
    let layout = regions(frame.area(), app.show_sidebar);

    if let Some(area) = layout.sidebar {
        sidebar::render(frame, app, area);
    }
    if let Some(area) = layout.editor {
        editor_view::render(frame, app, area);
    }

    statusbar::render(frame, app, layout.status);

    if app.input_mode != InputMode::Normal {
        popup::render(frame, app);
    }
}
