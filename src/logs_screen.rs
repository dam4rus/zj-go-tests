use zellij_tile::prelude::*;

#[derive(Debug)]
pub(crate) enum UpdateResult {
    ExitScreen,
}

#[derive(Debug, Default)]
pub(crate) struct LogsScreen {
    logs: Vec<String>,
}

impl LogsScreen {
    pub(crate) fn new(logs: Vec<String>) -> Self {
        Self { logs }
    }

    pub(crate) fn update(&mut self, event: Event) -> Option<UpdateResult> {
        match event {
            Event::Key(Key::Esc) => Some(UpdateResult::ExitScreen),
            _ => None,
        }
    }

    pub(crate) fn render(&mut self, rows: usize, cols: usize) {
        for (y, item) in self.logs.iter().enumerate() {
            print_text_with_coordinates(Text::new(item), 0, y, Some(cols), Some(rows));
        }
    }
}
