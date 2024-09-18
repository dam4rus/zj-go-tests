use zellij_tile::prelude::*;

use crate::{logs_screen::LogsScreen, Package, TestCase};

#[derive(Debug)]
pub(crate) enum UpdateCommand {
    ShowLogsScreen(LogsScreen),
    Render,
}

#[derive(Debug, Default)]
pub(crate) struct TestsScreen {
    pub(crate) packages: Vec<Package>,
    selected_index: usize,
    selected_index_changed: bool,
    scroll_y: usize,
}

impl TestsScreen {
    pub(crate) fn update(&mut self, event: Event) -> Option<UpdateCommand> {
        match event {
            Event::Key(Key::Down | Key::Char('j')) => {
                self.selected_index = self
                    .selected_index
                    .saturating_add(1)
                    .min(self.test_count().saturating_sub(1));
                self.selected_index_changed = true;
                Some(UpdateCommand::Render)
            }
            Event::Key(Key::Up | Key::Char('k')) => {
                self.selected_index = self.selected_index.saturating_sub(1);
                self.selected_index_changed = true;
                Some(UpdateCommand::Render)
            }
            Event::Key(Key::Char('\n')) => {
                self.build_list_items()
                    .get(self.selected_index)
                    .map(|list_item| {
                        UpdateCommand::ShowLogsScreen(match list_item {
                            ListItem::Package(package) => LogsScreen::new(package.log.clone()),
                            ListItem::TestCase(test_case) => LogsScreen::new(test_case.log.clone()),
                        })
                    })
            }
            _ => None,
        }
    }

    pub(crate) fn render(&mut self, rows: usize, cols: usize) {
        let bottom_index = self.scroll_y + rows - 2;
        if self.selected_index > bottom_index {
            self.scroll_y = self
                .scroll_y
                .saturating_add(self.selected_index - bottom_index);
        } else if self.selected_index < self.scroll_y {
            self.scroll_y = self
                .scroll_y
                .saturating_sub(self.scroll_y - self.selected_index);
        }

        let table_rows = self.build_table_rows();
        let table = Table::new().add_row(vec!["package", "test", "result"]);

        let table = table_rows
            .into_iter()
            .enumerate()
            .skip(self.scroll_y)
            .take(rows - 1)
            .fold(table, |acc, (i, row)| {
                if i == self.selected_index {
                    acc.add_styled_row(
                        row.into_iter()
                            .map(|column| Text::new(column).selected())
                            .collect(),
                    )
                } else {
                    acc.add_row(row)
                }
            });
        print_table_with_coordinates(table, 0, 0, Some(cols), Some(rows));
    }

    fn test_count(&self) -> usize {
        self.packages
            .iter()
            .fold(0, |acc, package| acc + 1 + package.tests.len())
    }

    fn build_table_rows(&self) -> Vec<Vec<&str>> {
        self.packages.iter().fold(Vec::new(), |mut acc, package| {
            let mut row = Vec::new();
            row.push(package.name.as_str());
            row.push(" ");
            row.push(
                package
                    .result
                    .as_ref()
                    .map(|result| result.as_ref())
                    .unwrap_or(" "),
            );
            acc.push(row);
            for test in &package.tests {
                let mut row = Vec::new();
                row.push(package.name.as_str());
                row.push(test.name.as_str());
                row.push(
                    test.result
                        .as_ref()
                        .map(|result| result.as_ref())
                        .unwrap_or(" "),
                );
                acc.push(row);
            }
            acc
        })
    }

    fn build_list_items(&self) -> Vec<ListItem> {
        self.packages
            .iter()
            .flat_map(|package| {
                let mut list_items = Vec::new();
                list_items.push(ListItem::Package(package));
                for test in &package.tests {
                    list_items.push(ListItem::TestCase(test));
                }
                list_items
            })
            .collect()
    }
}

#[derive(Debug)]
enum ListItem<'a> {
    Package(&'a Package),
    TestCase(&'a TestCase),
}
