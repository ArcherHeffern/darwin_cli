use std::{
    collections::HashSet,
    fs::{remove_dir_all, remove_file, OpenOptions},
    io::{stdin, stdout, Write},
    path::Path,
};

use crate::{
    anonomize, clean, config::darwin_root, create_darwin, create_report, download_results, list_students::{self}, plagiarism_checker, project_runner::Project, run_tests::{self}, types::TestResultError, util::{prompt_digit, prompt_yn}, view_student_results, view_student_submission
};

pub fn create_darwin(
    project: &Project,
    project_skeleton: &Path,
    moodle_submissions_zipfile: &Path,
    copy_ignore_set: &HashSet<&str>,
) -> bool {
    if darwin_root().exists() {
        if !prompt_yn("Darwin project already exists in this directory. Override? (y/n)")
            .unwrap_or(false)
        {
            return false;
        }
        if remove_dir_all(darwin_root()).is_err() {
            eprintln!("Failed to delete darwin project");
            return false;
        }
    }
    if let Err(e) = create_darwin::create_darwin(
        project,
        project_skeleton,
        moodle_submissions_zipfile,
        copy_ignore_set,
    ) {
        eprintln!("Error while creating darwin project: {}", e);
        return false;
    }
    true
}

pub fn auto(
    project: &Project,
    project_skeleton: &Path,
    moodle_submissions_zipfile: &Path,
    copy_ignore_set: &HashSet<&str>,
) {
    // TODO: Allow user to make mistakes when inputting using prompt_digit
    if !create_darwin(
        project,
        project_skeleton,
        moodle_submissions_zipfile,
        copy_ignore_set,
    ) {
        return;
    }

    let tests: Vec<String> = project.list_tests().iter().cloned().collect();
    let selected_tests = auto_select_tests(&tests).unwrap();
    if selected_tests.is_empty() {
        eprintln!("No tests selected. Exiting...");
        return;
    }

    let num_threads = prompt_digit::<usize>(
        "How many threads would you like to use while running test? (Recommended 4)",
    )
    .inspect_err(|e| {
        println!("{}", e);
    })
    .unwrap(); // TODO: How to fix this control flow

    let num_threads = num_threads.clamp(1, 8);

    for selected_test in selected_tests.iter() {
        println!("Running test: {}", selected_test);
        run_tests(project, selected_test, num_threads);
    }

    let num_sections = match prompt_digit::<usize>("How many TA's will be grading? This will determine how many sections the report will be split into.") {
        Ok(n) => {
            if n == 666 {
                eprintln!("✝️");
            }
            n
        }
        Err(_) => {
            let mut n_sections = 0;
            loop {
                if let Ok(r) = prompt_digit::<usize>("...") {
                    n_sections = r;
                    break;
                }
            }
            n_sections
        }
    };

    create_report(project, Path::new("report"), num_sections as u8, &selected_tests);
    plagiarism_check(Path::new("plagiarism.html"));
}

fn auto_select_tests(tests: &[String]) -> std::io::Result<Vec<String>> {
    println!("Which Tests would you like to run? [index, done, quit]");
    let mut selected_indeces: HashSet<usize> = HashSet::new();
    let mut buf = String::new();
    loop {
        println!("TESTS");
        for (i, test) in tests.iter().enumerate() {
            if selected_indeces.contains(&i) {
                println!("{}: ✅ {}", i, test);
            } else {
                println!("{}: ❌ {}", i, test);
            }
        }

        buf.clear();
        print!("next|exit|index >>> ");
        stdout().flush().expect("Failed to flush stdout");
        match stdin().read_line(&mut buf) {
            Err(e) => {
                eprintln!("Failed to read stdin: {}", e);
                return Err(e);
            }
            Ok(_) => {
                let buf = buf[..buf.len() - 1].to_lowercase();
                if buf == "next" {
                    break;
                } else if buf == "exit" {
                    return Ok(Vec::new());
                } else {
                    match str::parse::<usize>(&buf) {
                        Err(_) => {}
                        Ok(index) => {
                            if index >= tests.len() {
                                continue;
                            } else if selected_indeces.contains(&index) {
                                selected_indeces.remove(&index);
                            } else {
                                selected_indeces.insert(index);
                            }
                        }
                    }
                }
            }
        }
        println!();
    }
    let selected_tests: Vec<String> = selected_indeces
        .iter()
        .map(|index| tests[*index].clone())
        .collect();
    Ok(selected_tests)
}
pub fn list_students() {
    for student in list_students::list_students() {
        println!("{}", student);
    }
}

pub fn list_tests(project: &Project) {
    for test in project.list_tests() {
        println!("{}", test);
    }
}

pub fn run_test_for_student(project: &Project, student: &str, test: &str) {
    match run_tests::run_test_for_student(project, student, test) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}

pub fn run_tests(project: &Project, test: &str, num_threads: usize) {
    match run_tests::concurrent_run_tests(
        project,
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

pub enum ViewMode {
    Summarize,
    ClassName,
    Everything,
}

pub fn view_student_result(project: &Project, student: &str, test: &str, view_mode: &ViewMode) {
    match view_student_results::parse_test_results(project, student, test) {
        Ok(result) => match view_mode {
            ViewMode::Summarize => {
                println!("{}", result.summarize());
            }
            ViewMode::ClassName => {
                println!("{}", result.print());
            }
            ViewMode::Everything => {
                println!("{}", result.everything());
            }
        },
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

pub fn view_all_results(project: &Project, test: &str, summarize: &ViewMode) {
    if !project.list_tests().contains(test) {
        eprintln!("Test '{}' not recognized", test);
        return;
    }
    list_students::list_students().iter().for_each(|student| {
        println!("Processing '{}'", student);
        view_student_result(project, student, test, summarize);
    });
}

pub fn download_results_summary(project: &Project, test: &str, outfile: &str) {
    let out_file_path = Path::new(outfile);
    if out_file_path.exists()
        && !prompt_yn(&format!("{} Exists. Continue? (y/n)", outfile)).unwrap_or(false)
    {
        return;
    }
    let out_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(out_file_path)
        .unwrap();
    download_results::download_results_summary(project, out_file, test).unwrap();
}
pub fn download_results_by_classname(project: &Project, test: &str, outfile: &str) {
    let out_file = Path::new(outfile);
    if out_file.exists()
        && !prompt_yn(&format!("{} Exists. Continue? (y/n)", outfile)).unwrap_or(false)
    {
        return;
    }
    download_results::download_results_by_classname(project, out_file, test).unwrap();
}

pub fn view_student_submission(student: &str) {
    let dest = Path::new(student);
    if dest.exists()
        && !prompt_yn(&format!("'{}' Exists. Continue? (Y/N)", student)).unwrap_or(false)
    {
        println!("Aborting...");
        return;
    }
    if (dest.is_file() && remove_file(dest).is_err())
        || (dest.is_dir() && remove_dir_all(dest).is_err())
    {
        eprintln!("Failed to remove {:?}", dest);
        return;
    }

    if let Err(e) = view_student_submission::view_student_submission(student, dest) {
        eprintln!("Error viewing student submission: {}", e);
    };
}

pub fn create_report(project: &Project, report_path: &Path, parts: u8, tests: &Vec<String>) -> bool {
    if report_path.exists()
        && !prompt_yn(&format!("{:?} Exists. Continue? (y/n)", report_path)).unwrap_or(false)
    {
        return false;
    }

    if parts == 0 {
        eprintln!("Cannot split report into 0 parts");
        return false;
    }

    if (report_path.is_file() && remove_file(report_path).is_err())
        || (report_path.is_dir() && remove_dir_all(report_path).is_err())
    {
        eprintln!("Failed to remove {:?}", report_path);
        return false;
    }

    match create_report::create_report(project, report_path, tests, parts) {
        Ok(()) => {
            println!("Report generated at {:?}", report_path);
            true
        }
        Err(e) => {
            eprintln!("Error generating report: {}", e);
            false
        }
    }
}

pub fn plagiarism_check(dest_path: &Path) {
    if dest_path.exists()
        && !prompt_yn(&format!("{:?} Exists. Continue? (y/n)", dest_path)).unwrap_or(false)
    {
        return;
    }

    if (dest_path.is_file() && remove_file(dest_path).is_err())
        || (dest_path.is_dir() && remove_dir_all(dest_path).is_err())
    {
        eprintln!("Failed to remove {:?}", dest_path);
        return;
    }

    match plagiarism_checker::plagiarism_check(dest_path) {
        Ok(_) => {
            println!("Plagiarism report generated at {:?}", dest_path);
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}

pub fn plagiarism_check_students(student1: String, student2: String) {
    match plagiarism_checker::plagiarism_check_students(&student1, &student2) {
        Ok(score) => {
            println!(
                "{} and {} have a similarity score of {}",
                student1, student2, score
            );
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}

pub fn clean() {
    if let Err(e) = clean::clean() {
        eprintln!("Error cleaning: {}", e);
    }
}

pub fn anonomize(project: &Project) {
    anonomize::anonomize(project);
    println!("Done");
}
