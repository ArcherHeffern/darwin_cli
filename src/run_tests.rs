use std::{
    fs::{remove_dir_all, remove_file, rename, OpenOptions}, io::{self, prelude::*, ErrorKind}, path::Path, process::{Command, Stdio}
};

use crate::util::{self, initialize_project, is_student, is_test, set_active_project};

pub fn run_test_for_student(darwin_path: &Path, project_path: &Path, student: &str, test: &str) -> io::Result<()> {
    assert!(darwin_path.is_dir());

    if !is_test(darwin_path, test) {
        return Err(io::Error::new(ErrorKind::InvalidInput, format!("Test '{}' was not found", test)));
    }
    if !is_student(darwin_path, student) {
        return Err(io::Error::new(ErrorKind::InvalidInput, format!("Student '{}' was not found", student)));
    }
    let results_filename_to = format!("{}_{}", student, test);
    if darwin_path.join("results").join(results_filename_to).is_file() {
        return Ok(());
    }
    
    if project_path.is_dir() {
        remove_dir_all(project_path)?;
    } else if project_path.is_file() {
        remove_file(project_path)?;
    }

    if util::file_contains_line(darwin_path.join("tests_ran").as_path(), test)? {
        return Ok(())
    }

    match util::find_student_diff_file(darwin_path, student).take() {
        Some(diff_path) => {
            // Ensure tests are valid tests
            match process_diff_tests(
                darwin_path,
                project_path,
                student,
                test,
                &Path::new(&diff_path),
            ) {
                Err(e) => Err(e),
                Ok(()) => Ok(())
            }
        }
        None => {
            Err(io::Error::new(ErrorKind::NotFound, format!("This should not be possible. Perhaps you deleted '{}' diff file?", student)))
        }
    }
}
// Run test
// Parse test results
fn process_diff_tests(
    darwin_path: &Path,
    project_path: &Path,
    student: &str,
    test: &str,
    diff_path: &Path,
) -> Result<(), io::Error> {
    // Invariants
    // - darwin_path is a darwin project
    // - project_path does not exist
    // - student is a valid student
    // - test is a valid test
    // - diff_path exists
    initialize_project(darwin_path, project_path)?;
    set_active_project(darwin_path, project_path, diff_path)?;
    if let Err(e) = compile(darwin_path) {
        let compile_error_path = darwin_path.join("results").join("compile_errors");
        let mut compile_error_file = OpenOptions::new().read(true).append(true).open(compile_error_path).unwrap();

        compile_error_file.write(format!("{}\n", student).as_bytes())?;
        return Err(e);
    }
    run_test(project_path, test)?;
    relocate_test_results(darwin_path, project_path, student, test)?;
    remove_dir_all(project_path)?;

    return Ok(())
}

fn compile(darwin_path: &Path) -> Result<(), io::Error> {
    // Assume the student diff has already been resolved and placed into .darwin/project/src/main
    // mvn compile

    let mut compile_command = Command::new("mvn")
        .current_dir(darwin_path.join("project"))
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
    assert!(project_path.is_dir());

    let mut run_tests_command = Command::new("mvn")
        .current_dir(project_path)
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

fn relocate_test_results(darwin_path: &Path, project_path: &Path, student: &str, test: &str) -> Result<(), io::Error> {
    let results_filename_from = format!("TEST-{}.xml", test);
    let results_file_from = project_path.join("target").join("surefire-reports").join(results_filename_from);
    let results_filename_to = format!("{}_{}", student, test);
    let results_file_to = darwin_path.join("results").join(results_filename_to);
    rename(results_file_from, results_file_to).unwrap();
    Ok(())
}
