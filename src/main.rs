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

struct Package<'a> {
    name: &'a str,
    result: Option<TestResult>,
    tests: Vec<TestCase<'a>>,
    log: Vec<&'a str>,
}

struct TestCase<'a> {
    name: &'a str,
    result: Option<TestResult>,
    log: Vec<&'a str>,
}

#[derive(Debug, Default)]
struct GoTests {
    lines: Vec<TestLine>,
}

impl ZellijPlugin for GoTests {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {}

    fn update(&mut self, _event: Event) -> bool {
        false
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        if let Some(payload) = pipe_message.payload {
            let line: TestLine =
                serde_json::from_str(&payload).expect("Failed to deserialize Go test line json");
            self.lines.push(line);
            true
        } else {
            false
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        let mut packages = Vec::new();
        for line in &self.lines {
            match line.action {
                Some(Action::Start) => packages.push(Package {
                    name: line
                        .package
                        .as_deref()
                        .expect("Expected name for package in `Start` action"),
                    result: None,
                    tests: Vec::new(),
                    log: Vec::new(),
                }),
                Some(action @ (Action::Skip | Action::Pass | Action::Fail)) => {
                    if let Some(package) = packages.iter_mut().find(|package| {
                        package.name
                            == line.package.as_deref().expect(&format!(
                                "Expected name for package in `{}` action",
                                action.as_ref()
                            ))
                    }) {
                        if let Some(test) = package
                            .tests
                            .iter_mut()
                            .find(|test| line.test.as_deref() == Some(test.name))
                        {
                            test.result = Some(action.try_into().unwrap())
                        } else {
                            package.result = Some(action.try_into().unwrap());
                        }
                    }
                }
                Some(Action::Run) => {
                    if let Some(package) = packages.iter_mut().find(|package| {
                        package.name
                            == line
                                .package
                                .as_deref()
                                .expect("Expected name for package in `Run` action")
                    }) {
                        package.tests.push(TestCase {
                            name: line.test.as_deref().expect("Expected test name"),
                            result: None,
                            log: Vec::new(),
                        });
                    }
                }
                Some(Action::Output) => {
                    if let Some(test_case) = &line.test {
                        if let Some(test) = packages
                            .iter_mut()
                            .find(|package| {
                                package.name
                                    == line
                                        .package
                                        .as_deref()
                                        .expect("Expected name for package in `Output` action")
                            })
                            .and_then(|package| {
                                package.tests.iter_mut().find(|test| test.name == test_case)
                            })
                        {
                            test.log.push(
                                line.output
                                    .as_deref()
                                    .expect("Expected output in `Output` action"),
                            );
                        }
                    } else {
                        if let Some(package) = packages.iter_mut().find(|package| {
                            package.name
                                == line
                                    .package
                                    .as_deref()
                                    .expect("Expected name for package in `Output` action")
                        }) {
                            package.log.push(
                                line.output
                                    .as_deref()
                                    .expect("Expected output in `Output` action"),
                            );
                        }
                    }
                }
                _ => (),
            }
        }
        let table_rows = packages.iter().fold(Vec::new(), |mut acc, package| {
            let mut row = Vec::new();
            row.push(package.name);
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
                row.push(package.name);
                row.push(test.name);
                row.push(
                    test.result
                        .as_ref()
                        .map(|result| result.as_ref())
                        .unwrap_or(" "),
                );
                acc.push(row);
            }
            acc
        });
        let table = Table::new().add_row(vec!["package", "test", "result"]);
        let table = table_rows
            .into_iter()
            .skip(0)
            .fold(table, |acc, row| acc.add_row(row));
        print_table_with_coordinates(table, 0, 0, Some(cols), Some(rows));
    }
}

register_plugin!(GoTests);
