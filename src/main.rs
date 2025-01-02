use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
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
mod clean;

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
    Clean
}

#[derive(Debug)]
struct TestResults {
    student: String,
    test: String,
    results: Vec<TestResult>
}

impl TestResults {
    fn summary(&self) -> (usize, usize, usize) {
        // Correct, errored, failed
        let num_correct = self.results.iter().filter(|r|r.msg == StatusMsg::None).count();
        let num_errored = self.results.iter().filter(|r|matches!(r.msg, StatusMsg::Error {..})).count();
        let num_failed = self.results.iter().filter(|r|matches!(r.msg, StatusMsg::Failure{..})).count();
        (num_correct, num_errored, num_failed)
    }
    fn summarize(&self) -> String {
        let summary = self.summary();
        format!("{}_{}: Correct: {}, Errored: {}, Failed: {}", self.student, self.test, summary.0, summary.1, summary.2)
    }

    fn summarize_by_classname(&self) -> HashMap<String, (i32, i32, i32)> {
        let mut m: HashMap<String, (i32, i32, i32)> = HashMap::new();
        for result in self.results.iter() {
            if !m.contains_key(&result.classname) {
                m.insert(result.classname.clone(), (0, 0, 0));
            }
            let index = match &result.msg {
                StatusMsg::None => 0,
                StatusMsg::Error { .. } => 1,
                StatusMsg::Failure { .. } => 2
            };
            if let Some(entry) = m.get_mut(&result.classname) {
                match index {
                    0 => entry.0 += 1,
                    1 => entry.1 += 1,
                    2 => entry.2 += 1,
                    _ => {}  // In case of an unexpected index, do nothing
                }
            }
        }
        m
    }

    fn print(&self) -> String {
        let m = self.summarize_by_classname();
        format!("{}_{} {:?}",self.student, self.test, m)
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct TestResult {
    name: String,
    classname: String,
    time: Duration,
    msg: StatusMsg,
}


#[derive(Debug)]
#[derive(PartialEq)]
enum StatusMsg {
    None,
    Failure {
        message: Option<String>,
        type_: String, 
    },
    Error {
        message: Option<String>,
        type_: String
    }
}

fn main() {
    let mut copy_ignore_set = HashSet::new();
    copy_ignore_set.insert(".DS_Store");
    copy_ignore_set.insert(".gitignore");

    let darwin_path: &Path = Path::new(".darwin");

    let cli = Args::parse();

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
                darwin_path,
                project_skeleton.as_std_path(),
                moodle_submissions_zipfile.as_std_path(),
                &copy_ignore_set,
            );
        }
        SubCommand::DeleteProject => {
            fs::remove_dir_all(darwin_path).unwrap();
        }
        SubCommand::ListTests => {
            commands::list_tests(darwin_path);
        }
        SubCommand::ListStudents => {
            commands::list_students(darwin_path);
        }
        SubCommand::TestStudent { student, tests } => {
            commands::run_test_for_student(
                darwin_path,
                student.as_str(),
                tests.as_str(),
            );
        },
        SubCommand::TestAll { tests, num_threads } => {
            commands::run_tests(
                darwin_path,
                tests.as_str(),
                num_threads.unwrap_or(1)
            )
        }
        SubCommand::ViewStudentResultSummary { student, test } => {
            commands::view_student_result(darwin_path, &student, &test, true);
        }
        SubCommand::ViewStudentResultByClassName { student, test } => {
            commands::view_student_result(darwin_path, &student, &test, false);
        }
        SubCommand::ViewAllStudentsResultsSummary { test } => {
            commands::view_all_results(darwin_path, test.as_str(), true);
        }
        SubCommand::ViewAllStudentsResultsByClassName { test } => {
            commands::view_all_results(darwin_path, test.as_str(), false);
        }
        SubCommand::DownloadResultsSummary { test, outfile } => {
            commands::download_results_summary(darwin_path, test.as_str(), outfile.as_str());
        }
        SubCommand::DownloadResultsByClassName { test, outfile } => {
            commands::download_results_by_classname(darwin_path, test.as_str(), outfile.as_str());
        }
        SubCommand::ViewStudentSubmission { student } => {
            commands::view_student_submission(darwin_path, student.as_str());
        }
        SubCommand::Clean  => {
            commands::clean(darwin_path);
        }
    }
}
