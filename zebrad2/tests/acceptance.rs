//! Acceptance test: runs the application as a subprocess and asserts its
//! output for given argument combinations matches what is expected.
//!
//! For more information, see:
//! <https://docs.rs/abscissa/latest/abscissa/testing/index.html>

use abscissa::testing::CmdRunner;

#[test]
fn start_no_args() {
    let mut cmd = CmdRunner::default().arg("start").capture_stdout().run();
    cmd.stdout().expect_line("Hello, world!");
    cmd.wait().unwrap().expect_success();
}

#[test]
fn start_with_args() {
    let mut cmd = CmdRunner::default()
        .args(&["start", "acceptance", "test"])
        .capture_stdout()
        .run();

    cmd.stdout().expect_line("Hello, acceptance test!");
    cmd.wait().unwrap().expect_success();
}
