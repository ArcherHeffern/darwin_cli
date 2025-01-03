use std::{
    fs::{remove_dir_all, remove_file, rename, OpenOptions},
    io::{self, prelude::*, Error, ErrorKind, Result},
    path::Path,
    process::{Command, Stdio},
};
use threadpool::ThreadPool;

use crate::util::{initialize_project, is_student, is_test, set_active_project, to_diff_path};

pub fn concurrent_run_tests(
    darwin_path: &Path,
    test: &str,
    num_threads: usize,
    on_thread_start: fn(&str),
    on_thread_err: fn(&str, Error),
    on_thread_end: fn(&str),
) -> Result<()> {
    if !is_test(darwin_path, test) {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            format!("Test {} not recognized", test),
        ));
    }

    _concurrent_run_tests(
        darwin_path,
        test,
        num_threads,
        on_thread_start,
        on_thread_err,
        on_thread_end,
    )
}

fn _concurrent_run_tests(
    darwin_path: &Path,
    test: &str,
    num_threads: usize,
    on_thread_start: fn(&str),
    on_thread_err: fn(&str, Error),
    on_thread_end: fn(&str),
) -> io::Result<()> {
    let threadpool = ThreadPool::new(num_threads);

    for diff_path in darwin_path.join("submission_diffs").read_dir()? {
        let diff_path = diff_path.unwrap();
        let student = diff_path.file_name().into_string().expect("?");
        let darwin_path_clone = darwin_path.to_path_buf();
        let test_clone = test.to_string();
        threadpool.execute(move || {
            on_thread_start(&student);
            // let darwin_path_clone = darwin_path.to_pat
            match run_test_for_student(darwin_path_clone.as_path(), &student, &test_clone) {
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
    let mut f = OpenOptions::new()
        .append(true)
        .open(darwin_path.join("tests_ran"))?;
    writeln!(f, "{}", test)?;
    Ok(())
}

pub fn run_test_for_student(darwin_path: &Path, student: &str, test: &str) -> Result<()> {
    // Validate Inputs
    if !darwin_path.is_dir() {
        return Err(Error::new(
            ErrorKind::AlreadyExists,
            "darwin project not initialized in this directory",
        ));
    }
    if !is_test(darwin_path, test) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Test '{}' was not found", test),
        ));
    }
    if !is_student(darwin_path, student) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Student '{}' was not found", student),
        ));
    }

    // Validate state of .darwin and perform clean up
    let project_path = Path::new(darwin_path).join("projects").join(student);
    if project_path.is_dir() {
        remove_dir_all(&project_path)?;
    } else if project_path.is_file() {
        remove_file(&project_path)?;
    }

    // Don't recompute
    let dest_file = darwin_path
        .join("results")
        .join(format!("{}_{}", student, test));
    if dest_file.exists() {
        return Ok(());
    }

    _run_test_for_student(
        darwin_path,
        project_path.as_path(),
        student,
        test,
        dest_file.as_path(),
    )
}

fn _run_test_for_student(
    darwin_path: &Path,
    project_path: &Path,
    student: &str,
    test: &str,
    dest_file: &Path,
) -> Result<()> {
    let diff_path = to_diff_path(darwin_path, student);
    initialize_project(project_path)?;
    set_active_project(project_path, diff_path.as_path())?;
    if let Err(e) = compile(project_path) {
        let compile_error_path = darwin_path.join("results").join("compile_errors");
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

fn relocate_test_results(
    project_path: &Path,
    test: &str,
    dest_file: &Path
) -> Result<()> {
    let results_filename_from = format!("TEST-{}.xml", test);
    let results_file_from = project_path
        .join("target")
        .join("surefire-reports")
        .join(results_filename_from);
    rename(results_file_from, dest_file).unwrap();
    Ok(())
}
