use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum::AsRefStr;
use zellij_tile::prelude::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, AsRefStr)]
#[serde(rename_all = "lowercase")]
enum Action {
    Start,
    Run,
    Output,
    Pass,
    Fail,
    Skip,
}

#[derive(Debug, Clone, Copy, AsRefStr)]
#[strum(serialize_all = "lowercase")]
enum TestResult {
    Pass,
    Fail,
    Skip,
}

impl TryFrom<Action> for TestResult {
    type Error = String;

    fn try_from(value: Action) -> Result<Self, Self::Error> {
        match value {
            Action::Pass => Ok(TestResult::Pass),
            Action::Fail => Ok(TestResult::Fail),
            Action::Skip => Ok(TestResult::Skip),
            action => Err(format!(
                "Action `{}` is not a valid TestResult",
                action.as_ref()
            )),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "PascalCase"))]
struct TestLine {
    action: Option<Action>,
    package: Option<String>,
    test: Option<String>,
    output: Option<String>,
}

#[derive(Debug, Clone)]
struct Package {
    name: String,
    result: Option<TestResult>,
    tests: Vec<TestCase>,
    log: Vec<String>,
}

#[derive(Debug, Clone)]
struct TestCase {
    name: String,
    result: Option<TestResult>,
    log: Vec<String>,
}

#[derive(Debug, Default)]
struct GoTestsPlugin {
    packages: Vec<Package>,
    selected_index: usize,
    selected_index_changed: bool,
    scroll_y: usize,
}

impl ZellijPlugin for GoTestsPlugin {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        subscribe(&[EventType::Key])
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Key(Key::Down | Key::Char('j')) => {
                self.selected_index = self
                    .selected_index
                    .saturating_add(1)
                    .min(self.test_count().saturating_sub(1));
                self.selected_index_changed = true;
                true
            }
            Event::Key(Key::Up | Key::Char('k')) => {
                self.selected_index = self.selected_index.saturating_sub(1);
                self.selected_index_changed = true;
                true
            }
            _ => false,
        }
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        if let Some(payload) = pipe_message.payload {
            let line: TestLine =
                serde_json::from_str(&payload).expect("Failed to deserialize Go test line json");
            match line.action {
                Some(Action::Start) => self.packages.push(Package {
                    name: line
                        .package
                        .expect("Expected name for package in `Start` action"),
                    result: None,
                    tests: Vec::new(),
                    log: Vec::new(),
                }),
                Some(action @ (Action::Skip | Action::Pass | Action::Fail)) => {
                    if let Some(package) = self.packages.iter_mut().find(|package| {
                        package.name
                            == line.package.as_deref().expect(&format!(
                                "Expected name for package in `{}` action",
                                action.as_ref()
                            ))
                    }) {
                        if let Some(test) = package
                            .tests
                            .iter_mut()
                            .find(|test| line.test.as_deref() == Some(&test.name))
                        {
                            test.result = Some(action.try_into().unwrap())
                        } else {
                            package.result = Some(action.try_into().unwrap());
                        }
                    }
                }
                Some(Action::Run) => {
                    if let Some(package) = self.packages.iter_mut().find(|package| {
                        package.name
                            == line
                                .package
                                .as_deref()
                                .expect("Expected name for package in `Run` action")
                    }) {
                        package.tests.push(TestCase {
                            name: line.test.expect("Expected test name"),
                            result: None,
                            log: Vec::new(),
                        });
                    }
                }
                Some(Action::Output) => {
                    if let Some(test_case) = &line.test {
                        if let Some(test) = self
                            .packages
                            .iter_mut()
                            .find(|package| {
                                package.name
                                    == line
                                        .package
                                        .as_deref()
                                        .expect("Expected name for package in `Output` action")
                            })
                            .and_then(|package| {
                                package
                                    .tests
                                    .iter_mut()
                                    .find(|test| test.name == *test_case)
                            })
                        {
                            test.log
                                .push(line.output.expect("Expected output in `Output` action"));
                        }
                    } else {
                        if let Some(package) = self.packages.iter_mut().find(|package| {
                            package.name
                                == line
                                    .package
                                    .as_deref()
                                    .expect("Expected name for package in `Output` action")
                        }) {
                            package
                                .log
                                .push(line.output.expect("Expected output in `Output` action"));
                        }
                    }
                }
                _ => (),
            }
            true
        } else {
            false
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        let over_selection = rows - 2 + self.scroll_y;
        if self.selected_index > over_selection {
            self.scroll_y = self
                .scroll_y
                .saturating_add(self.selected_index - over_selection);
        } else if self.selected_index < self.scroll_y {
            self.scroll_y = self
                .scroll_y
                .saturating_sub(self.scroll_y - self.selected_index);
        }

        let table_rows = self.build_table_rows();
        let table = Table::new().add_row(vec!["package", "test", "result"]);

        let table =
            table_rows
                .into_iter()
                .enumerate()
                .skip(self.scroll_y)
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
}

impl GoTestsPlugin {
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
}

register_plugin!(GoTestsPlugin);
