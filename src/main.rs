use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
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
mod download_results;

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
        let num_correct = self.results.iter().filter(|r|r.msg == StatusMsg::NONE).count();
        let num_errored = self.results.iter().filter(|r|matches!(r.msg, StatusMsg::ERROR {..})).count();
        let num_failed = self.results.iter().filter(|r|matches!(r.msg, StatusMsg::FAILURE{..})).count();
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
                StatusMsg::NONE => 0,
                StatusMsg::ERROR { .. } => 1,
                StatusMsg::FAILURE { .. } => 2
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
            commands::run_test_for_student(
                project_path,
                student.as_str(),
                tests.as_str(),
            );
        },
        SubCommand::TestAll { tests } => {
            commands::run_tests(
                project_path,
                tests.as_str()
            )
        }
        SubCommand::ViewStudentResultSummary { student, test } => {
            commands::view_student_result(project_path, &student, &test, true);
        }
        SubCommand::ViewStudentResultByClassName { student, test } => {
            commands::view_student_result(project_path, &student, &test, false);
        }
        SubCommand::ViewAllStudentsResultsSummary { test } => {
            commands::view_all_results(project_path, test.as_str(), true);
        }
        SubCommand::ViewAllStudentsResultsByClassName { test } => {
            commands::view_all_results(project_path, test.as_str(), false);
        }
        SubCommand::DownloadResultsSummary { test, outfile } => {
            commands::download_results_summary(project_path, test.as_str(), outfile.as_str());
        }
        SubCommand::DownloadResultsByClassName { test, outfile } => {
            commands::download_results_by_classname(project_path, test.as_str(), outfile.as_str());
        }
        SubCommand::ViewStudentSubmission { student } => {
            commands::view_student_submission(project_path, student.as_str());
        }
    }
}
