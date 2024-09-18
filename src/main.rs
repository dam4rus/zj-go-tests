use logs_screen::LogsScreen;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum::AsRefStr;
use tests_screen::TestsScreen;
use zellij_tile::prelude::*;

mod logs_screen;
mod tests_screen;

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
    elapsed: Option<f64>,
}

#[derive(Debug, Clone)]
struct Package {
    name: String,
    result: Option<TestResult>,
    elapsed: Option<f64>,
    tests: Vec<TestCase>,
    log: Vec<String>,
}

#[derive(Debug, Clone)]
struct TestCase {
    name: String,
    result: Option<TestResult>,
    elapsed: Option<f64>,
    log: Vec<String>,
}

#[derive(Debug, Default)]
struct GoTestsPlugin {
    tests_screen: TestsScreen,
    logs_screen: Option<LogsScreen>,
}

impl ZellijPlugin for GoTestsPlugin {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        subscribe(&[EventType::Key])
    }

    fn update(&mut self, event: Event) -> bool {
        match &mut self.logs_screen {
            Some(logs_screen) => match logs_screen.update(event) {
                Some(logs_screen::UpdateResult::ExitScreen) => {
                    self.logs_screen = None;
                    true
                }
                None => false,
            },
            None => match self.tests_screen.update(event) {
                Some(tests_screen::UpdateCommand::Render) => true,
                Some(tests_screen::UpdateCommand::ShowLogsScreen(logs_screen)) => {
                    self.logs_screen = Some(logs_screen);
                    true
                }
                None => false,
            },
        }
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        if let Some(payload) = pipe_message.payload {
            let line: TestLine =
                serde_json::from_str(&payload).expect("Failed to deserialize Go test line json");
            match line.action {
                Some(Action::Start) => self.tests_screen.packages.push(Package {
                    name: line
                        .package
                        .expect("Expected name for package in `Start` action"),
                    result: None,
                    tests: Vec::new(),
                    log: Vec::new(),
                    elapsed: None,
                }),
                Some(action @ (Action::Skip | Action::Pass | Action::Fail)) => {
                    if let Some(package) = self.tests_screen.packages.iter_mut().find(|package| {
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
                            test.result = Some(action.try_into().unwrap());
                            test.elapsed = line.elapsed;
                        } else {
                            package.result = Some(action.try_into().unwrap());
                            package.elapsed = line.elapsed;
                        }
                    }
                }
                Some(Action::Run) => {
                    if let Some(package) = self.tests_screen.packages.iter_mut().find(|package| {
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
                            elapsed: None,
                        });
                    }
                }
                Some(Action::Output) => {
                    if let Some(test_case) = &line.test {
                        if let Some(test) = self
                            .tests_screen
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
                        if let Some(package) =
                            self.tests_screen.packages.iter_mut().find(|package| {
                                package.name
                                    == line
                                        .package
                                        .as_deref()
                                        .expect("Expected name for package in `Output` action")
                            })
                        {
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
        match &mut self.logs_screen {
            Some(logs_screen) => logs_screen.render(rows, cols),
            None => self.tests_screen.render(rows, cols),
        }
    }
}

register_plugin!(GoTestsPlugin);
