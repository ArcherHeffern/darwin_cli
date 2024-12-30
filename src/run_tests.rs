use std::{
    collections::HashSet, fs::{rename, File, OpenOptions}, io::{self, prelude::*}, path::Path, process::{Command, Stdio}
};

use crate::{util::set_active_project, TestResult};

// Run test
// Parse test results
pub fn process_diff_tests(
    project_path: &Path,
    student: &str,
    diff_path: &Path,
    tests: &str,
    copy_ignore_set: &HashSet<&str>,
) -> Result<(), io::Error> {
    // Assumes valid inputs
    set_active_project(project_path, diff_path, &copy_ignore_set)?;
    if let Err(e) = compile(project_path) {
        let compile_error_path = project_path.join("results").join("compile_errors");
        let mut compile_error_file = OpenOptions::new().read(true).append(true).open(compile_error_path).unwrap();

        compile_error_file.write(format!("{}\n", student).as_bytes())?;
        return Err(e);
    }
    run_test(project_path, tests)?;
    relocate_test_results(project_path, student, tests)?;

    return Ok(())
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
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "'mvn test-compile' failed"));
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

    run_tests_command.wait()?;
    // let status = run_tests_command.wait()?;
    // if !status.success() {
    //     eprintln!(
    //         "'mvn -Dtest={} surefire:test' failed with status: {}",
    //         test, status
    //     );
    // }

    Ok(())
}

fn relocate_test_results(project_path: &Path, student: &str, tests: &str) -> Result<(), io::Error> {
    for test in tests.split(',') {
        let results_filename_from = format!("TEST-{}.xml", test);
        let results_file_from = project_path.join("project").join("target").join("surefire-reports").join(results_filename_from);
        let results_filename_to = format!("{}_{}", student, test);
        let results_file_to = project_path.join("results").join(results_filename_to);
        rename(results_file_from, results_file_to).unwrap();

    }
    Ok(())
}
