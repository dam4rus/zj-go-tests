use std::ops::Range;

use zellij_tile::prelude::*;

#[derive(Debug)]
pub(crate) enum UpdateCommand {
    ExitScreen,
    Render,
}

#[derive(Debug, Default)]
pub(crate) enum Mode {
    #[default]
    Normal,
    Search(String),
}

#[derive(Debug, Default)]
pub(crate) struct LogsScreen {
    logs: Vec<String>,
    scroll_x: usize,
    scroll_y: usize,
    screen_width: Option<usize>,
    screen_height: Option<usize>,
    mode: Mode,
    search_result: Vec<(usize, Range<usize>)>,
}

impl LogsScreen {
    pub(crate) fn new(logs: Vec<String>) -> Self {
        Self {
            logs,
            ..Self::default()
        }
    }

    pub(crate) fn update(&mut self, event: Event) -> Option<UpdateCommand> {
        match &mut self.mode {
            Mode::Normal => match event {
                Event::Key(Key::Esc) => Some(UpdateCommand::ExitScreen),
                Event::Key(Key::Down | Key::Char('j')) => {
                    self.scroll_y = self
                        .scroll_y
                        .saturating_add(1)
                        .min(self.logs.len().saturating_sub(1));
                    Some(UpdateCommand::Render)
                }
                Event::Key(Key::Up | Key::Char('k')) => {
                    self.scroll_y = self.scroll_y.saturating_sub(1);
                    Some(UpdateCommand::Render)
                }
                Event::Key(Key::Left | Key::Char('h')) => {
                    self.scroll_x = self.scroll_x.saturating_sub(1);
                    Some(UpdateCommand::Render)
                }
                Event::Key(Key::Right | Key::Char('l')) => {
                    self.scroll_x = (self.scroll_x + 1).min(1);
                    Some(UpdateCommand::Render)
                }
                Event::Key(Key::PageDown | Key::Char('d')) => {
                    if let Some(height) = self.screen_height {
                        self.scroll_y = self
                            .scroll_y
                            .saturating_add(height / 2)
                            .min(self.logs.len().saturating_sub(1));
                    }
                    Some(UpdateCommand::Render)
                }
                Event::Key(Key::PageUp | Key::Char('u')) => {
                    if let Some(height) = self.screen_height {
                        self.scroll_y = self.scroll_y.saturating_sub(height / 2);
                    }
                    Some(UpdateCommand::Render)
                }
                Event::Key(Key::Char('/')) => {
                    self.mode = Mode::Search(String::new());
                    Some(UpdateCommand::Render)
                }
                _ => None,
            },
            Mode::Search(search_string) => match event {
                Event::Key(Key::Esc) | Event::Key(Key::Char('\n')) => {
                    self.mode = Mode::Normal;
                    Some(UpdateCommand::Render)
                }
                Event::Key(Key::Char(c)) => {
                    search_string.push(c);
                    self.search_result = self
                        .logs
                        .iter()
                        .enumerate()
                        .flat_map(|(idx, line)| {
                            line.match_indices(search_string.as_str())
                                .map(|(start_idx, needle)| (idx, start_idx..(needle.len())))
                                .collect::<Vec<(usize, Range<usize>)>>()
                        })
                        .collect();
                    if let [head, ..] = &self.search_result[..] {
                        self.scroll_y = head.0
                    }
                    Some(UpdateCommand::Render)
                }
                _ => None,
            },
        }
    }

    pub(crate) fn render(&mut self, rows: usize, cols: usize) {
        self.screen_width = Some(cols);
        self.screen_height = Some(rows);
        for (y, item) in self
            .logs
            .iter()
            .skip(self.scroll_y)
            .take(rows - 2)
            .enumerate()
        {
            print_text_with_coordinates(Text::new(item), 0, y, Some(cols), Some(1));
        }

        match &self.mode {
            Mode::Search(search_string) => {
                print_text_with_coordinates(
                    Text::new(format!("/{}", search_string)),
                    0,
                    rows - 1,
                    Some(cols),
                    Some(1),
                );
            }
            _ => (),
        }
    }
}
