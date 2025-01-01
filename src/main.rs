use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use std::path::Path;
use std::time::Duration;
use std::{collections::HashSet, fs};

mod commands;
mod create_project;
mod run_tests;
mod list_students;
mod list_tests;
mod util;
mod view_student_results;

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
    ViewStudentResult {
        student: String,
        test: String,
    },
    ViewAllStudentsResults,
    DownloadResults,
}

#[derive(Debug)]
struct TestResults {
    student: String,
    test: String,
    results: Vec<TestResult>
}

impl TestResults {
    fn summarize(&self) -> String {
        let num_correct = self.results.iter().filter(|r|r.msg == StatusMsg::NONE).count();
        let num_failed = self.results.iter().filter(|r|matches!(r.msg, StatusMsg::FAILURE{..})).count();
        let num_errored = self.results.iter().filter(|r|matches!(r.msg, StatusMsg::ERROR {..})).count();
        format!("{}_{}: Correct: {}, Failed: {}, Errored: {}", self.student, self.test, num_correct, num_failed, num_errored)
    }
}

#[derive(Debug)]
struct TestResult {
    name: String,
    classname: String,
    time: Duration,
    msg: StatusMsg,
}


#[derive(Debug)]
#[derive(PartialEq)]
enum StatusMsg {
    NONE,
    FAILURE {
        message: Option<String>,
        type_: String, 
    },
    ERROR {
        message: Option<String>,
        type_: String
    }
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
            commands::run_tests_for_student(
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
        SubCommand::ViewStudentResult { student, test } => {
            commands::view_student_result(project_path, &student, &test);
        }
        _ => {
            todo!("Rest of commands");
        }
    }
}
