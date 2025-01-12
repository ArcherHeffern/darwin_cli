use std::{
    fs::{remove_dir_all, remove_file, rename, OpenOptions},
    io::{self, prelude::*, Error, ErrorKind, Result},
    path::Path,
    process::{Command, Stdio},
};
use threadpool::ThreadPool;

use crate::{
    config::{
        compile_errors_file, darwin_root, diff_dir, student_diff_file, student_project_file,
        student_result_file, tests_ran_file,
    }, project_runner::maven_project, util::{file_append_line, is_student, is_test}
};

pub fn concurrent_run_tests(
    test: &str,
    num_threads: usize,
    on_thread_start: fn(&str),
    on_thread_err: fn(&str, Error),
    on_thread_end: fn(&str),
) -> Result<()> {
    if !is_test(test) {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            format!("Test {} not recognized", test),
        ));
    }

    _concurrent_run_tests(
        test,
        num_threads,
        on_thread_start,
        on_thread_err,
        on_thread_end,
    )
}

fn _concurrent_run_tests(
    test: &str,
    num_threads: usize,
    on_thread_start: fn(&str),
    on_thread_err: fn(&str, Error),
    on_thread_end: fn(&str),
) -> io::Result<()> {
    let threadpool = ThreadPool::new(num_threads);

    for diff_path in diff_dir().read_dir()? {
        let diff_path = diff_path.unwrap();
        let student = diff_path.file_name().into_string().expect("?");
        let test_clone = test.to_string();
        threadpool.execute(move || {
            on_thread_start(&student);
            // let darwin_path_clone = darwin_path.to_pat
            match run_test_for_student(&student, &test_clone) {
                Ok(()) => {
                    on_thread_end(&student);
                }
                Err(e) => {
                    on_thread_err(&student, e);
                }
            }
        })
    }
    threadpool.join();
    file_append_line(&tests_ran_file(), test)
}

pub fn run_test_for_student(student: &str, test: &str) -> Result<()> {
    // Validate Inputs
    if !darwin_root().is_dir() {
        return Err(Error::new(
            ErrorKind::AlreadyExists,
            "darwin project not initialized in this directory",
        ));
    }
    if !is_test(test) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Test '{}' was not found", test),
        ));
    }
    if !is_student(student) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Student '{}' was not found", student),
        ));
    }

    // Validate state of .darwin and perform clean up
    let student_project_path = student_project_file(student);
    if student_project_path.is_dir() {
        remove_dir_all(&student_project_path)?;
    } else if student_project_path.is_file() {
        remove_file(&student_project_path)?;
    }

    // Don't recompute
    let dest_file = student_result_file(student, test);
    if dest_file.exists() {
        return Ok(());
    }

    _run_test_for_student(
        student_project_path.as_path(),
        student,
        test,
        dest_file.as_path(),
    )
}

fn _run_test_for_student(
    project_path: &Path,
    student: &str,
    test: &str,
    dest_file: &Path,
) -> Result<()> {
    let diff_path = student_diff_file(student);
    maven_project().recreate_normalized_project(project_path, &diff_path)?;
    if let Err(e) = compile(project_path) {
        let compile_error_path = compile_errors_file();
        let mut compile_error_file = OpenOptions::new()
            .read(true)
            .append(true)
            .open(compile_error_path)
            .unwrap();

        compile_error_file.write_all(format!("{}\n", student).as_bytes())?;
        remove_dir_all(project_path)?;
        return Err(e);
    }
    run_test(project_path, test)?;
    relocate_test_results(project_path, test, dest_file)?;
    remove_dir_all(project_path)?;

    Ok(())
}

fn compile(project_path: &Path) -> Result<()> {
    // mvn compile

    let mut compile_command = Command::new("mvn")
        .current_dir(project_path)
        .arg("test-compile")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let status = compile_command.wait()?;
    if !status.success() {
        return Err(Error::new(ErrorKind::Other, "'mvn test-compile' failed"));
    }

    Ok(())
}

fn run_test(project_path: &Path, test: &str) -> Result<()> {
    // mvn -Dtest={test_str} surefire:test

    let mut run_tests_command = Command::new("mvn")
        .current_dir(project_path)
        .arg(format!("-Dtest={}", test))
        .arg("surefire:test")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    run_tests_command.wait()?;
    Ok(())
}

fn relocate_test_results(project_path: &Path, test: &str, dest_file: &Path) -> Result<()> {
    let results_filename_from = format!("TEST-{}.xml", test);
    let results_file_from = project_path
        .join("target")
        .join("surefire-reports")
        .join(results_filename_from);
    rename(results_file_from, dest_file).unwrap();
    Ok(())
}
