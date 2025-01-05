use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use config::darwin_root;
use std::path::{Path, PathBuf};
use std::{collections::HashSet, fs};

mod commands;
mod create_darwin;
mod run_tests;
mod list_students;
mod list_tests;
mod util;
mod view_student_results;
mod download_results;
mod view_student_submission;
mod create_report;
mod clean;
mod config;
mod types;


#[derive(Parser, Debug)]
#[command(name = "Darwin", version = "1.0", author = "Archer Heffern", about = "Auto grader for Maven projects submitted to Moodle")]
struct Cli {
    /// Name of the person to greet
    #[command(subcommand)]
    command: SubCommand,
}

#[derive(Debug, Subcommand)]
enum SubCommand {
    CreateProject {
        project_skeleton: Utf8PathBuf,
        moodle_submissions_zipfile: Utf8PathBuf,
    },
    DeleteProject,
    ListStudents,
    ListTests,
    ViewStudentSubmission {
        student: String,
    },
    TestStudent {
        student: String,
        tests: String, // Must be comma separated list of tests
    },
    TestAll {
        tests: String,
        num_threads: Option<usize>
    },
    ViewStudentResultSummary {
        student: String,
        test: String,
    },
    ViewStudentResultByClassName {
        student: String,
        test: String
    },
    ViewAllStudentsResultsSummary {
        test: String
    },
    ViewAllStudentsResultsByClassName {
        test: String
    },
    DownloadResultsSummary {
        test: String,
        outfile: String
    },
    DownloadResultsByClassName {
        test: String,
        outfile: String
    },
    CreateReport {
        dest_path: Utf8PathBuf,
        parts: u8,
        tests: Vec<String>
    },
    Clean
}


fn main() {
    let mut copy_ignore_set = HashSet::new();
    copy_ignore_set.insert(".DS_Store");
    copy_ignore_set.insert(".gitignore");

    let darwin_path: PathBuf = config::darwin_root();
    let darwin_path: &Path = darwin_path.as_path();

    let cli = Cli::parse();

    let command = cli.command;
    if let SubCommand::CreateProject { .. } = command {
    } else if !darwin_path.exists() {
        eprintln!("create project first");
        return;
    }

    match command {
        SubCommand::CreateProject {
            project_skeleton,
            moodle_submissions_zipfile,
        } => {
            commands::create_darwin(
                project_skeleton.as_std_path(),
                moodle_submissions_zipfile.as_std_path(),
                &copy_ignore_set,
            );
        }
        SubCommand::DeleteProject => {
            fs::remove_dir_all(darwin_root()).unwrap();
        }
        SubCommand::ListTests => {
            commands::list_tests();
        }
        SubCommand::ListStudents => {
            commands::list_students();
        }
        SubCommand::TestStudent { student, tests } => {
            commands::run_test_for_student(
                student.as_str(),
                tests.as_str(),
            );
        },
        SubCommand::TestAll { tests, num_threads } => {
            commands::run_tests(
                tests.as_str(),
                num_threads.unwrap_or(1)
            )
        }
        SubCommand::ViewStudentResultSummary { student, test } => {
            commands::view_student_result(&student, &test, true);
        }
        SubCommand::ViewStudentResultByClassName { student, test } => {
            commands::view_student_result(&student, &test, false);
        }
        SubCommand::ViewAllStudentsResultsSummary { test } => {
            commands::view_all_results(test.as_str(), true);
        }
        SubCommand::ViewAllStudentsResultsByClassName { test } => {
            commands::view_all_results(test.as_str(), false);
        }
        SubCommand::DownloadResultsSummary { test, outfile } => {
            commands::download_results_summary(test.as_str(), outfile.as_str());
        }
        SubCommand::DownloadResultsByClassName { test, outfile } => {
            commands::download_results_by_classname(test.as_str(), outfile.as_str());
        }
        SubCommand::ViewStudentSubmission { student } => {
            commands::view_student_submission(student.as_str());
        }
        SubCommand::CreateReport { dest_path, parts, tests } => {
            commands::create_report(dest_path.as_std_path(), parts, &tests);
        }
        SubCommand::Clean  => {
            commands::clean();
        }
    }
}
