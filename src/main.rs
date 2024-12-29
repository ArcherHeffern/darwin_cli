use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use std::path::Path;
use std::{collections::HashSet, fs};

mod commands;
mod create_project;
mod run_tests;
mod util;

/*
TODO:
- Find tests by full directory instead of just file name. What if there are multiple files with same basename?
- Validate create-project input is dir and zipfile
*/

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[clap(subcommand)]
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
    },
    ViewResult {
        student: String,
    },
    ViewResults,
    DownloadResults,
}

#[derive(Debug)]
struct TestResult {
    correct: i32,
}

fn main() {
    let mut copy_ignore_set = HashSet::new();
    copy_ignore_set.insert(".DS_Store");
    copy_ignore_set.insert(".gitignore");

    let project_path: &Path = Path::new(".darwin");

    let cli = Args::parse();

    let command = cli.command;
    if let SubCommand::CreateProject { .. } = command {
    } else if !project_path.exists() {
        eprintln!("create project first");
        return;
    }

    match command {
        SubCommand::CreateProject {
            project_skeleton,
            moodle_submissions_zipfile,
        } => {
            commands::create_darwin(
                project_path,
                project_skeleton.as_std_path(),
                moodle_submissions_zipfile.as_std_path(),
                &copy_ignore_set,
            );
        }
        SubCommand::DeleteProject => {
            fs::remove_dir_all(project_path).unwrap();
        }
        SubCommand::ListTests => {
            commands::list_tests(project_path);
        }
        SubCommand::ListStudents => {
            commands::list_students(project_path);
        }
        SubCommand::TestStudent { student, tests } => {
            commands::run_test(
                project_path,
                student.as_str(),
                tests.as_str(),
                &copy_ignore_set,
            );
        },
        SubCommand::TestAll { tests } => {
            commands::run_tests(
                project_path,
                tests.as_str(),
                &copy_ignore_set,
            )
        }
        _ => {
            todo!("Rest of commands");
        }
    }
}
