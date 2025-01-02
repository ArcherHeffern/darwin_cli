use std::{
    collections::HashSet,
    fs::{create_dir, remove_dir, remove_file, OpenOptions},
    io::{stdin, BufRead},
    path::Path,
    process::exit,
};

use crate::{
    clean, create_darwin, download_results,
    list_students::{self},
    list_tests,
    run_tests::{self},
    util::{patch, prompt_yn},
    view_student_results::{self, TestResultError},
};

pub fn create_darwin(
    darwin_path: &Path,
    project_skeleton: &Path,
    moodle_submissions_zipfile: &Path,
    copy_ignore_set: &HashSet<&str>,
) {
    if darwin_path.exists() {
        if prompt_yn("Darwin project already exists in this directory. Override? (y/n)")
            .unwrap_or(false)
        {
            return;
        }
        if remove_dir(darwin_path).is_err() {
            eprintln!("Failed to delete darwin project");
            return;
        }
    }
    if let Err(e) = create_darwin::create_darwin(
        darwin_path,
        project_skeleton,
        moodle_submissions_zipfile,
        copy_ignore_set,
    ) {
        eprintln!("Error while creating darwin project: {}", e);
    }
}

pub fn list_students(darwin_path: &Path) {
    for student in list_students::list_students(darwin_path) {
        println!("{}", student);
    }
}

pub fn list_tests(darwin_path: &Path) {
    for test in crate::list_tests::list_tests(darwin_path) {
        println!("{}", test);
    }
}

pub fn run_test_for_student(darwin_path: &Path, student: &str, test: &str) {
    match run_tests::run_test_for_student(darwin_path, student, test) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}

pub fn run_tests(darwin_path: &Path, test: &str, num_threads: usize) {
    match run_tests::concurrent_run_tests(
        darwin_path,
        test,
        num_threads,
        |s| println!("Processing: {}", s),
        |s, e| eprintln!("Error processing {}: {}", s, e),
        |_| {},
    ) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}

pub fn view_student_result(darwin_path: &Path, student: &str, test: &str, summarize: bool) {
    match view_student_results::parse_test_results(darwin_path, student, test) {
        Ok(result) => {
            if summarize {
                println!("{}", result.summarize());
            } else {
                println!("{}", result.print());
            }
        }
        Err(e) => match e {
            TestResultError::IOError(er) => {
                eprintln!("{}", er);
            }
            TestResultError::CompilationError => {
                eprintln!("Compilation Error");
            }
            TestResultError::TestsNotRun => {
                eprintln!("Tests have not been run for this student");
            }
        },
    }
}

pub fn view_all_results(darwin_path: &Path, test: &str, summarize: bool) {
    if !list_tests::list_tests(darwin_path).contains(test) {
        eprintln!("Test '{}' not recognized", test);
        return;
    }
    list_students::list_students(darwin_path)
        .iter()
        .for_each(|student| {
            println!("Processing '{}'", student);
            view_student_result(darwin_path, student, test, summarize);
        });
}

pub fn download_results_summary(darwin_path: &Path, test: &str, outfile: &str) {
    let out_file_path = Path::new(outfile);
    if out_file_path.exists() {
        println!("{} Exists. Continue? (Y/N)", outfile);
        let mut s = String::new();
        stdin().lock().read_line(&mut s).expect("Stdin to work");
        s = s.to_lowercase();
        if s != "y\n" {
            exit(0);
        }
    }
    let out_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(out_file_path)
        .unwrap();
    download_results::download_results_summary(darwin_path, out_file, test).unwrap();
}
pub fn download_results_by_classname(darwin_path: &Path, test: &str, outfile: &str) {
    let out_file_path = Path::new(outfile);
    if out_file_path.exists() {
        println!("{} Exists. Continue? (Y/N)", outfile);
        let mut s = String::new();
        stdin().lock().read_line(&mut s).expect("Stdin to work");
        s = s.to_lowercase();
        if s != "y\n" {
            exit(0);
        }
    }
    let out_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(out_file_path)
        .unwrap();
    download_results::download_results_by_classname(darwin_path, out_file, test).unwrap();
}

pub fn view_student_submission(darwin_path: &Path, student: &str) {
    if !list_students::list_students(darwin_path)
        .iter()
        .any(|s| s == student)
    {
        eprintln!("Student '{}' does not exist\n", student);
        exit(1);
    }
    let student_diff_path = darwin_path.join("submission_diffs").join(student);
    if !student_diff_path.is_file() {
        eprintln!(
            "Student diff '{}' does not exist or is not a file",
            student_diff_path.to_string_lossy()
        );
        exit(1);
    }
    let dest = Path::new(student);
    if dest.exists() {
        println!("'{}' Exists. Continue? (Y/N)", student);
        let mut s = String::new();
        stdin().lock().read_line(&mut s).expect("Stdin to work");
        s = s.to_lowercase();
        if s != "y\n" {
            exit(0);
        }
    }
    if dest.is_file() {
        remove_file(dest).expect(&format!("Can remove file '{}'", student));
    } else if dest.is_dir() {
        remove_dir(dest).expect(&format!("Can remove directory '{}'", student));
    }

    create_dir(dest).expect(&format!("Can create directory '{}", student));
    patch(
        darwin_path.join("main").as_path(),
        student_diff_path.as_path(),
        dest,
    )
    .unwrap();
}

pub fn clean(darwin_path: &Path) {
    if let Err(e) = clean::clean(darwin_path) {
        eprintln!("Error cleaning: {}", e);
    }
}
