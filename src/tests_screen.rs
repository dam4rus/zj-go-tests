use zellij_tile::prelude::*;

use crate::{logs_screen::LogsScreen, Package, TestCase, TestResult};

#[derive(Debug)]
pub(crate) enum UpdateCommand {
    ShowLogsScreen(LogsScreen),
    Render,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ResultFilters {
    pass: bool,
    fail: bool,
    skip: bool,
}

#[derive(Debug, Default)]
pub(crate) struct TestsScreen {
    pub(crate) packages: Vec<Package>,
    selected_index: usize,
    scroll_x: usize,
    scroll_y: usize,
    result_filters: ResultFilters,
}

impl TestsScreen {
    pub(crate) fn update(&mut self, event: Event) -> Option<UpdateCommand> {
        match event {
            Event::Key(Key::Down | Key::Char('j')) => {
                self.selected_index = self
                    .selected_index
                    .saturating_add(1)
                    .min(self.visible_list_items().len().saturating_sub(1));
                Some(UpdateCommand::Render)
            }
            Event::Key(Key::Up | Key::Char('k')) => {
                self.selected_index = self.selected_index.saturating_sub(1);
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
            Event::Key(Key::Char('\n')) => {
                self.visible_list_items()
                    .get(self.selected_index)
                    .map(|list_item| {
                        UpdateCommand::ShowLogsScreen(match list_item {
                            ListItem::Package(package) => LogsScreen::new(package.log.clone()),
                            ListItem::TestCase(test_case) => LogsScreen::new(test_case.log.clone()),
                        })
                    })
            }
            Event::Key(Key::Char('1')) => {
                self.result_filters.pass = !self.result_filters.pass;
                Some(UpdateCommand::Render)
            }
            Event::Key(Key::Char('2')) => {
                self.result_filters.fail = !self.result_filters.fail;
                Some(UpdateCommand::Render)
            }
            Event::Key(Key::Char('3')) => {
                self.result_filters.skip = !self.result_filters.skip;
                Some(UpdateCommand::Render)
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

        let table_rows = self.render_list_items();
        let headers = ["package", "elapsed"];
        let table = Table::new().add_row(Vec::from(&headers[self.scroll_x..]));

        let table = table_rows
            .into_iter()
            .enumerate()
            .skip(self.scroll_y)
            .take(rows - 2)
            .fold(table, |acc, (i, row)| {
                if i == self.selected_index {
                    acc.add_styled_row(
                        row.into_iter()
                            .skip(self.scroll_x)
                            .map(|column| column.selected())
                            .collect(),
                    )
                } else {
                    acc.add_styled_row(row.into_iter().skip(self.scroll_x).collect())
                }
            });
        print_table_with_coordinates(table, 0, 0, Some(cols), Some(rows));
        let pass_ribbon = Text::new("[1] pass");
        let fail_ribbon = Text::new("[2] fail");
        let skip_ribbon = Text::new("[3] skip");
        print_ribbon_with_coordinates(
            if self.result_filters.pass {
                pass_ribbon.selected()
            } else {
                pass_ribbon
            },
            0,
            rows - 1,
            None,
            None,
        );
        print_ribbon_with_coordinates(
            if self.result_filters.fail {
                fail_ribbon.selected()
            } else {
                fail_ribbon
            },
            13,
            rows - 1,
            None,
            None,
        );
        print_ribbon_with_coordinates(
            if self.result_filters.skip {
                skip_ribbon.selected()
            } else {
                skip_ribbon
            },
            26,
            rows - 1,
            None,
            None,
        );
    }

    fn render_list_items(&self) -> Vec<Vec<Text>> {
        let list_items = self.visible_list_items();
        list_items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                item.render(
                    list_items
                        .get(i + 1)
                        .map(|next_item| {
                            if let ListItem::Package(_) = next_item {
                                true
                            } else {
                                false
                            }
                        })
                        .unwrap_or(true),
                )
            })
            .collect()
    }

    fn visible_list_items(&self) -> Vec<ListItem> {
        self.packages
            .iter()
            .filter(|package| self.is_test_visible(package.result.unwrap_or_default()))
            .flat_map(|package| {
                let mut list_items = Vec::new();
                list_items.push(ListItem::Package(package));
                for test in package
                    .tests
                    .iter()
                    .filter(|test| self.is_test_visible(test.result.unwrap_or_default()))
                {
                    list_items.push(ListItem::TestCase(test));
                }
                list_items
            })
            .collect()
    }

    fn is_test_visible(&self, test_result: TestResult) -> bool {
        matches!(
            (self.result_filters, test_result),
            (
                ResultFilters {
                    pass: false,
                    fail: false,
                    skip: false,
                },
                _,
            ) | (ResultFilters { pass: true, .. }, TestResult::Pass)
                | (ResultFilters { fail: true, .. }, TestResult::Fail)
                | (ResultFilters { skip: true, .. }, TestResult::Skip)
        )
    }
}

#[derive(Debug)]
enum ListItem<'a> {
    Package(&'a Package),
    TestCase(&'a TestCase),
}

impl<'a> ListItem<'a> {
    fn render(&self, is_last_element: bool) -> Vec<Text> {
        let mut row = Vec::new();
        match self {
            ListItem::Package(package) => {
                let (color, result) = package
                    .result
                    .unwrap_or(TestResult::Skip)
                    .marker_color_and_char();
                row.push(Text::new(format!("{} {}", result, package.name)).color_range(color, ..1));
                row.push(
                    package
                        .elapsed
                        .map(|elapsed| Text::new(format!("{}s", elapsed)))
                        .unwrap_or(Text::new(" ")),
                );
            }
            ListItem::TestCase(test_case) => {
                let border = if is_last_element { '└' } else { '├' };
                let (color, result) = test_case
                    .result
                    .unwrap_or(TestResult::Skip)
                    .marker_color_and_char();
                row.push(
                    Text::new(format!("{} {} {}", border, result, test_case.name))
                        .color_range(color, 2..3),
                );
                row.push(
                    test_case
                        .elapsed
                        .map(|elapsed| Text::new(format!("{}s", elapsed)))
                        .unwrap_or(Text::new(" ")),
                );
            }
        }
        row
    }
}
