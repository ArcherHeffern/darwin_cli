use std::{collections::HashSet, io, path::Path, process::{Command, Stdio}};

use crate::{util::set_active_project, TestResult};


pub fn process_diff_tests(project_path: &Path, diff_path: &Path, test: &str, copy_ignore_set: &HashSet<&str>) -> Result<TestResult, io::Error> {
    set_active_project(project_path, diff_path, &copy_ignore_set)?;
    compile(project_path)?;
    run_test(project_path, test)?;

    parse_test_results(project_path)
}

fn compile(project_path: &Path) -> Result<(), io::Error> {
    // Assume the student diff has already been resolved and placed into .darwin/project/src/main
    // mvn compile

    let mut compile_command = Command::new("mvn")
        .current_dir(project_path.join("project")) 
        .arg("test-compile")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let status = compile_command.wait()?;
    if !status.success() {
        eprintln!("'mvn test-compile' failed with status: {}", status);
    }

    Ok(())
}

fn run_test(project_path: &Path, test: &str) -> Result<(), io::Error> {
    // Assume the student diff has already been resolved and placed into .darwin/project/src/main
    // mvn -Dtest={test_str} surefire:test
    let mut run_tests_command = Command::new("mvn")
        .current_dir(project_path.join("project"))
        .arg(format!("-Dtest={}", test))
        .arg("surefire:test")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let status = run_tests_command.wait()?;
    if !status.success() {
        eprintln!("'mvn -Dtest={} surefire:test' failed with status: {}", test, status);
    }

    Ok(())
}

fn parse_test_results(project_path: &Path) -> Result<TestResult, io::Error> {
    Ok(TestResult { correct: 0 })
}