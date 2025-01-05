use std::{
    collections::HashSet,
    fs::{remove_dir_all, remove_file, OpenOptions},
    path::Path,
};

use crate::{
    clean,
    config::darwin_root,
    create_darwin, create_report, download_results,
    list_students::{self},
    list_tests,
    run_tests::{self},
    types::TestResultError,
    util::prompt_yn,
    view_student_results, view_student_submission,
};

pub fn create_darwin(
    project_skeleton: &Path,
    moodle_submissions_zipfile: &Path,
    copy_ignore_set: &HashSet<&str>,
) {
    if darwin_root().exists() {
        if !prompt_yn("Darwin project already exists in this directory. Override? (y/n)")
            .unwrap_or(false)
        {
            return;
        }
        if remove_dir_all(darwin_root()).is_err() {
            eprintln!("Failed to delete darwin project");
            return;
        }
    }
    if let Err(e) = create_darwin::create_darwin(
        project_skeleton,
        moodle_submissions_zipfile,
        copy_ignore_set,
    ) {
        eprintln!("Error while creating darwin project: {}", e);
    }
}

pub fn list_students() {
    for student in list_students::list_students() {
        println!("{}", student);
    }
}

pub fn list_tests() {
    for test in crate::list_tests::list_tests() {
        println!("{}", test);
    }
}

pub fn run_test_for_student(student: &str, test: &str) {
    match run_tests::run_test_for_student(student, test) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}

pub fn run_tests(test: &str, num_threads: usize) {
    match run_tests::concurrent_run_tests(
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

pub fn view_student_result(student: &str, test: &str, summarize: bool) {
    match view_student_results::parse_test_results(student, test) {
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
            TestResultError::TestsNotRun => {
                eprintln!("Tests have not been run for this student");
            }
        },
    }
}

pub fn view_all_results(test: &str, summarize: bool) {
    if !list_tests::list_tests().contains(test) {
        eprintln!("Test '{}' not recognized", test);
        return;
    }
    list_students::list_students().iter().for_each(|student| {
        println!("Processing '{}'", student);
        view_student_result(student, test, summarize);
    });
}

pub fn download_results_summary(test: &str, outfile: &str) {
    let out_file_path = Path::new(outfile);
    if out_file_path.exists() {
        if !prompt_yn(&format!("{} Exists. Continue? (y/n)", outfile)).unwrap_or(false) {
            return;
        }
    }
    let out_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(out_file_path)
        .unwrap();
    download_results::download_results_summary(out_file, test).unwrap();
}
pub fn download_results_by_classname(test: &str, outfile: &str) {
    let out_file = Path::new(outfile);
    if out_file.exists() {
        if !prompt_yn(&format!("{} Exists. Continue? (y/n)", outfile)).unwrap_or(false) {
            return;
        }
    }
    download_results::download_results_by_classname(out_file, test).unwrap();
}

pub fn view_student_submission(student: &str) {
    let dest = Path::new(student);
    if dest.exists() {
        if !prompt_yn(&format!("'{}' Exists. Continue? (Y/N)", student)).unwrap_or(false) {
            println!("Aborting...");
            return;
        }
    }
    if dest.is_file() && remove_file(dest).is_err() {
        eprintln!("Failed to remove {:?}", dest);
        return;
    } else if dest.is_dir() && remove_dir_all(dest).is_err() {
        eprintln!("Failed to remove {:?}", dest);
        return;
    }

    if let Err(e) = view_student_submission::view_student_submission(student, dest) {
        eprintln!("Error viewing student submission: {}", e);
    };
}

pub fn create_report(report_path: &Path, parts: u8, tests: &Vec<String>) {
    if report_path.exists() {
        if !prompt_yn(&format!("{:?} Exists. Continue? (y/n)", report_path)).unwrap_or(false) {
            return;
        }
    }

    if parts == 0 {
        eprintln!("Cannot split report into 0 parts");
        return;
    }

    if report_path.is_file() && remove_file(report_path).is_err() {
        eprintln!("Failed to remove {:?}", report_path);
        return;
    } else if report_path.is_dir() && remove_dir_all(report_path).is_err() {
        eprintln!("Failed to remove {:?}", report_path);
        return;
    }

    match create_report::create_report(report_path, tests, parts) {
        Ok(()) => {
            println!("Report generated");
        }
        Err(e) => {
            eprintln!("Error generating report: {}", e);
        }
    }
}

pub fn clean() {
    if let Err(e) = clean::clean() {
        eprintln!("Error cleaning: {}", e);
    }
}
