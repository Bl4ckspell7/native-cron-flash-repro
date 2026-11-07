// A GUI-subsystem program has no console of its own, so every console program
// it starts without CREATE_NO_WINDOW gets a new console window.
#![windows_subsystem = "windows"]

//! Calls native-cron from a GUI program, one button per call, so the console
//! windows its schtasks.exe and powershell.exe children open become visible.

use std::thread;

use native_cron::{CronOptions, WindowsOptions};

slint::slint! {
    import { Button, TextEdit, VerticalBox, HorizontalBox } from "std-widgets.slint";

    export component AppWindow inherits Window {
        title: "native-cron flash repro";
        preferred-width: 760px;
        preferred-height: 440px;

        in-out property <string> log;
        in property <bool> busy;
        callback run(string);

        VerticalBox {
            Text {
                text: "Each button calls native-cron from this GUI-subsystem program. Watch for console windows.";
                wrap: word-wrap;
            }
            HorizontalBox {
                Button {
                    text: "Register (23:59)";
                    enabled: !root.busy;
                    clicked => { root.run("register"); }
                }
                Button {
                    text: "Status";
                    enabled: !root.busy;
                    clicked => { root.run("status"); }
                }
                Button {
                    text: "Remove";
                    enabled: !root.busy;
                    clicked => { root.run("remove"); }
                }
                Button {
                    text: "Register → Status → Remove";
                    enabled: !root.busy;
                    clicked => { root.run("all"); }
                }
            }
            HorizontalBox {
                Button {
                    text: "Register for 00:00 (already passed today)";
                    enabled: !root.busy;
                    clicked => { root.run("register-passed"); }
                }
                Text {
                    text: "Task Scheduler runs the job right away: whoami.exe opens a console window.";
                    vertical-alignment: center;
                    wrap: word-wrap;
                }
            }
            TextEdit {
                read-only: true;
                text: root.log;
                vertical-stretch: 1;
            }
        }
    }
}

const NAMESPACE: &str = "native-cron-flash-repro";
const JOB: &str = "demo";
/// Late enough that today's run is still ahead, so registering starts nothing.
const AHEAD: &str = "59 23 * * *";
/// Passed every day but at midnight itself: Task Scheduler catches it up.
const PASSED: &str = "0 0 * * *";

/// A daily job that only prints the user name; "Remove" deletes it again.
fn options(cron: &str) -> CronOptions {
    CronOptions::new(JOB, cron, [r"C:\Windows\System32\whoami.exe"])
        .namespace(NAMESPACE)
        .overwrite(true)
        .windows(WindowsOptions { visible: true })
}

fn steps(action: &str) -> &'static [&'static str] {
    match action {
        "register" => &["register"],
        "register-passed" => &["register-passed"],
        "status" => &["status"],
        "remove" => &["remove"],
        _ => &["register", "status", "remove"],
    }
}

/// Runs one native-cron call and describes its result.
fn call(step: &str) -> String {
    let result = match step {
        "register" => {
            native_cron::register(options(AHEAD)).map(|_| "registered for 23:59".to_string())
        }
        "register-passed" => {
            native_cron::register(options(PASSED)).map(|_| "registered for 00:00".to_string())
        }
        "status" => native_cron::job_in(NAMESPACE, JOB)
            .and_then(|job| job.status())
            .map(|status| format!("state {:?}", status.state)),
        _ => native_cron::remove_in(NAMESPACE, JOB).map(|()| "removed".to_string()),
    };
    match result {
        Ok(text) => format!("{step}: {text}"),
        Err(error) => format!("{step}: error: {error}"),
    }
}

fn append(app: &slint::Weak<AppWindow>, line: String) {
    let app = app.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(app) = app.upgrade() {
            app.set_log(format!("{}{line}\n", app.get_log()).into());
        }
    });
}

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    let weak = app.as_weak();
    app.on_run(move |action| {
        if let Some(app) = weak.upgrade() {
            app.set_busy(true);
        }
        let steps = steps(action.as_str());
        let weak = weak.clone();
        // native-cron blocks until schtasks/powershell exit: keep the UI responsive.
        thread::spawn(move || {
            for step in steps {
                append(&weak, call(step));
            }
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = weak.upgrade() {
                    app.set_busy(false);
                }
            });
        });
    });
    app.run()
}
