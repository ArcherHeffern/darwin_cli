use std::{
    fs::{remove_dir_all, remove_file},
    io::{self, Error, ErrorKind, Result},
    path::Path,
};
use threadpool::ThreadPool;

use crate::{
    config::{
        compile_errors_file, darwin_root, diff_dir, student_diff_file, student_project_file,
        student_result_file
    }, darwin_config::{read_config, write_config}, project_runner::Project, util::{file_append_line, is_student, is_test}
};

pub fn concurrent_run_test(
    project: &Project,
    test: &str,
    num_threads: usize,
    on_thread_start: fn(&str),
    on_thread_err: fn(&str, Error),
    on_thread_end: fn(&str),
) -> Result<()> {
    if !is_test(project, test) {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            format!("Test {} not recognized", test),
        ));
    }
    if read_config()?.tests_run.contains(&test.to_string()) {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            format!("Test {} already ran", test),
        ));
    }

    _concurrent_run_test(
        project,
        test,
        num_threads,
        on_thread_start,
        on_thread_err,
        on_thread_end,
    )
}

fn _concurrent_run_test(
    project: &Project,
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
        let project_copy = project.clone();
        threadpool.execute(move || {
            on_thread_start(&student);
            // let darwin_path_clone = darwin_path.to_pat
            match run_test_for_student(&project_copy, &student, &test_clone) {
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

    let mut config = read_config()?;
    config.tests_run.push(test.to_string());
    write_config(config)?;
    Ok(())
}

pub fn run_test_for_student(project: &Project, student: &str, test: &str) -> Result<()> {
    // Validate Inputs
    if !darwin_root().is_dir() {
        return Err(Error::new(
            ErrorKind::AlreadyExists,
            "darwin project not initialized in this directory",
        ));
    }
    if !is_test(project, test) {
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
        project,
        student_project_path.as_path(),
        student,
        test,
        dest_file.as_path(),
    )
}

fn _run_test_for_student(
    project: &Project,
    project_path: &Path,
    student: &str,
    test: &str,
    dest_file: &Path,
) -> Result<()> {
    let diff_path = student_diff_file(student);
    project.recreate_normalized_project(project_path, &diff_path)?;
    if let Err(e) = project.compile(project_path) {
        file_append_line(&compile_errors_file(), &format!("{}:{}", student, e.to_string()))?;
        remove_dir_all(project_path)?;
        return Err(e);
    }
    project.run_test(project_path, test)?;
    project.relocate_test_results(project_path, test, dest_file)?;
    remove_dir_all(project_path)?;

    Ok(())
}
