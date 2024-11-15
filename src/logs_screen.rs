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
    search_result: Search,
}

#[derive(Debug, Default)]
pub(crate) struct Search {
    matches: Vec<(usize, Range<usize>)>,
    current_index: Option<usize>,
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
                Event::Key(KeyWithModifier {
                    bare_key: BareKey::Esc,
                    ..
                }) => Some(UpdateCommand::ExitScreen),
                Event::Key(KeyWithModifier {
                    bare_key: BareKey::Down | BareKey::Char('j'),
                    ..
                }) => {
                    self.scroll_y = self
                        .scroll_y
                        .saturating_add(1)
                        .min(self.logs.len().saturating_sub(1));
                    Some(UpdateCommand::Render)
                }
                Event::Key(KeyWithModifier {
                    bare_key: BareKey::Up | BareKey::Char('k'),
                    ..
                }) => {
                    self.scroll_y = self.scroll_y.saturating_sub(1);
                    Some(UpdateCommand::Render)
                }
                Event::Key(KeyWithModifier {
                    bare_key: BareKey::Left | BareKey::Char('h'),
                    ..
                }) => {
                    self.scroll_x = self.scroll_x.saturating_sub(1);
                    Some(UpdateCommand::Render)
                }
                Event::Key(KeyWithModifier {
                    bare_key: BareKey::Right | BareKey::Char('l'),
                    ..
                }) => {
                    self.scroll_x = (self.scroll_x + 1).min(1);
                    Some(UpdateCommand::Render)
                }
                Event::Key(KeyWithModifier {
                    bare_key: BareKey::PageDown | BareKey::Char('d'),
                    ..
                }) => {
                    if let Some(height) = self.screen_height {
                        self.scroll_y = self
                            .scroll_y
                            .saturating_add(height / 2)
                            .min(self.logs.len().saturating_sub(1));
                    }
                    Some(UpdateCommand::Render)
                }
                Event::Key(KeyWithModifier {
                    bare_key: BareKey::PageUp | BareKey::Char('u'),
                    ..
                }) => {
                    if let Some(height) = self.screen_height {
                        self.scroll_y = self.scroll_y.saturating_sub(height / 2);
                    }
                    Some(UpdateCommand::Render)
                }
                Event::Key(KeyWithModifier {
                    bare_key: BareKey::Char('f'),
                    ..
                }) => {
                    if let Some(height) = self.screen_height {
                        self.scroll_y = self
                            .scroll_y
                            .saturating_add(height)
                            .min(self.logs.len().saturating_sub(1));
                    }
                    Some(UpdateCommand::Render)
                }
                Event::Key(KeyWithModifier {
                    bare_key: BareKey::Char('b'),
                    ..
                }) => {
                    if let Some(height) = self.screen_height {
                        self.scroll_y = self.scroll_y.saturating_sub(height);
                    }
                    Some(UpdateCommand::Render)
                }
                Event::Key(KeyWithModifier {
                    bare_key: BareKey::Char('/'),
                    ..
                }) => {
                    self.mode = Mode::Search(String::new());
                    Some(UpdateCommand::Render)
                }
                Event::Key(KeyWithModifier {
                    bare_key: BareKey::Char('n'),
                    ..
                }) => {
                    if let Some(current_index) = &mut self.search_result.current_index {
                        *current_index = current_index
                            .saturating_add(1)
                            .min(self.search_result.matches.len().saturating_sub(1));
                        self.scroll_y = self.search_result.matches[*current_index].0;
                        Some(UpdateCommand::Render)
                    } else {
                        None
                    }
                }
                Event::Key(KeyWithModifier {
                    bare_key: BareKey::Char('N'),
                    ..
                }) => {
                    if let Some(current_index) = &mut self.search_result.current_index {
                        *current_index = current_index.saturating_sub(1);
                        self.scroll_y = self.search_result.matches[*current_index].0;
                        Some(UpdateCommand::Render)
                    } else {
                        None
                    }
                }
                _ => None,
            },
            Mode::Search(search_string) => match event {
                Event::Key(KeyWithModifier {
                    bare_key: BareKey::Esc | BareKey::Enter,
                    ..
                }) => {
                    self.mode = Mode::Normal;
                    Some(UpdateCommand::Render)
                }
                Event::Key(KeyWithModifier {
                    bare_key: BareKey::Char(c),
                    ..
                }) => {
                    search_string.push(c);
                    self.search_result.matches = self
                        .logs
                        .iter()
                        .enumerate()
                        .flat_map(|(idx, line)| {
                            line.match_indices(search_string.as_str())
                                .map(|(start_idx, needle)| (idx, start_idx..(needle.len())))
                                .collect::<Vec<(usize, Range<usize>)>>()
                        })
                        .collect();
                    if let [head, ..] = &self.search_result.matches[..] {
                        self.scroll_y = head.0;
                        self.search_result.current_index = Some(0);
                    } else {
                        self.search_result.current_index = None;
                    }
                    Some(UpdateCommand::Render)
                }
                _ => None,
            },
        }
    }

    pub(crate) fn render(&mut self, rows: usize, cols: usize) {
        self.screen_width = Some(cols);
        self.screen_height = Some(rows - 1);
        for (y, item) in self
            .logs
            .iter()
            .skip(self.scroll_y)
            .take(self.screen_height.unwrap())
            .enumerate()
        {
            print_text_with_coordinates(Text::new(item), 0, y, Some(cols), Some(1));
        }

        let bottom_text = match &self.mode {
            Mode::Normal => Text::new(":"),
            Mode::Search(search_string) => Text::new(format!("/{}", search_string)),
        };

        print_text_with_coordinates(bottom_text, 0, rows - 1, Some(cols), Some(1));
    }
}
