use std::{
    borrow::Borrow, fs::{self, create_dir, remove_dir_all}, io::{Error, ErrorKind, Result}, path::Path
};

use serde::Serialize;
use tempfile::tempdir;
use handlebars::Handlebars;

use crate::{
    config::{darwin_root, student_diff_file, tests_ran_file}, list_students::list_students, list_tests::list_tests, types::{StatusMsg, TestResult, TestResultError, TestResults}, util::{
        file_contains_line, flatten_move_recursive, list_files_recursively, recreate_student_main,
    }, view_student_results::parse_test_results
};

pub fn create_report(report_path: &Path, tests: &Vec<String>) -> Result<()> {
    if !darwin_root().is_dir() {
        return Err(Error::new(
            ErrorKind::NotFound,
            "Darwin project not initialized",
        ));
    }
    if report_path.exists() {
        return Err(Error::new(ErrorKind::AlreadyExists, "report_path exists"));
    }
    if tests.is_empty() {
        return Err(Error::new(
            ErrorKind::AlreadyExists,
            "expected at least one test",
        ));
    }
    let actual_tests = list_tests();
    for test in tests {
        if !actual_tests.contains(test) {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("{} is not a test", test),
            ));
        }
        if !file_contains_line(&tests_ran_file(), test)? {
            println!(
                "Warning! {} is a test but it wasn't run for all students",
                test
            );
            // return Err(Error::new(ErrorKind::NotFound, format!("{} is a test but wasn't run for all students", test)))
        }
    }
    _create_report(report_path, tests).map_err(|e| {
        if report_path.exists() {
            remove_dir_all(report_path)
                .expect("Create report and deleting report directory during cleanup failed");
        }
        e
    })
}

fn _create_report(report_root: &Path, tests: &Vec<String>) -> Result<()> {
    report_initialize(report_root).map_err(|e| {
        Error::new(
            ErrorKind::Other,
            format!("Failed to initialize report: {}", e),
        )
    })?;
    let students = list_students();
    if students.is_empty() {
        return Ok(());
    }

    let mut handlebars = Handlebars::new();
    initialize_handlebars(&mut handlebars)?;

    create_report_student_list(&report_root.join("index.html"), &students, &handlebars)?;
    create_student_reports(report_root, tests, &students, &handlebars)?;

    Ok(())
}

fn initialize_handlebars(handlebars: &mut Handlebars) -> Result<()> {

    handlebars.register_template_string("student_list", include_str!("../template/index.hbs"))
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    handlebars.register_template_string("student_index_template", include_str!("../template/student_index.hbs"))
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    // handlebars.register_template_string("student_template", include_str!("../template/student.hbs"))
        // .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    Ok(())
}

#[derive(Serialize)]
struct StudentListContext<'a> {
    students: &'a [String],
}

fn report_initialize(report_root: &Path) -> Result<()> {
    create_dir(report_root)?;
    create_dir(report_root.join("students"))?;
    create_dir(report_root.join("styles"))?;
    
    fs::write(report_root.join("styles").join("global.css"), include_bytes!("../template/global.css"))?;
    fs::write(report_root.join("styles").join("index.css"), include_str!("../template/index.css"))?;
    fs::write(report_root.join("styles").join("student_index.css"), include_str!("../template/student_index.css"))?;
    fs::write(report_root.join("styles").join("student.css"), include_str!("../template/student.css"))?;
    fs::write(report_root.join("styles").join("LibreBaskerville-Regular.ttf"), include_bytes!("../template/LibreBaskerville-Regular.ttf"))?;
    fs::write(report_root.join("styles").join("LibreBaskerville-Italic.ttf"), include_bytes!("../template/LibreBaskerville-Italic.ttf"))?;
    fs::write(report_root.join("styles").join("LibreBaskerville-Bold.ttf"), include_bytes!("../template/LibreBaskerville-Bold.ttf"))?;
    fs::write(report_root.join("styles").join("OFL.txt"), include_bytes!("../template/OFL.txt"))?;
    Ok(())
}

fn create_report_student_list(dest: &Path, students: &[String], handlebars: &Handlebars) -> Result<()> {

    let rendered = handlebars.render("student_list", &StudentListContext {students}).map_err(|e|Error::new(ErrorKind::Other, format!("could not open template: {}", e)))?;
    fs::write(dest, rendered)
}

fn create_student_reports(
    report_root: &Path,
    tests: &Vec<String>,
    students: &[String],
    handlebars: &Handlebars
) -> Result<()> {

    let mut prev_student = "";
    for i in 0..students.len() - 1 {
        let student = students[i].as_str();
        create_student_report(
            report_root,
            tests,
            &prev_student,
            &student,
            &students[i + 1],
            &handlebars
        )?;
        prev_student = student;
    }
    create_student_report(
        report_root,
        tests,
        &prev_student,
        &students[students.len() - 1],
        "",
        &handlebars
    )?;
    Ok(())
}

fn create_student_report(
    report_root: &Path,
    tests: &Vec<String>,
    prev_student: &str,
    student: &str,
    next_student: &str,
    handlebars: &Handlebars
) -> Result<()> {
    _create_student_report(report_root, tests, prev_student, student, next_student, handlebars)
}

fn _create_student_report(
    report_root: &Path,
    tests: &Vec<String>,
    prev_student: &str,
    student: &str,
    next_student: &str,
    student_template: &Handlebars<'_>
) -> Result<()> {
    let diff_path = student_diff_file(student);
    let student_dir = &report_root.join("students").join(student);
    let tmpdir = tempdir()?;
    recreate_student_main(&diff_path, tmpdir.path(), tmpdir.path())?;
    let file_paths = list_files_recursively(tmpdir.path());

    let mut files = Vec::new();
    file_paths.iter().for_each(|file_path| {
        let mut html_path = file_path.clone();
        html_path.set_extension("html");
        let html_path = html_path.file_name().expect(&format!("File name should exist on {:?}", html_path));
        let html_path = html_path.to_string_lossy().to_string();
        let java_path = file_path.strip_prefix(tmpdir.path()).map_err(|_|Error::new(ErrorKind::Other, "Could not strip tmpdir path from file path")).unwrap();
        let java_path = java_path.to_string_lossy().to_string();
        files.push( StudentTemplateFile { html_path, java_path });
    });

    let mut test_packages: Vec<TestPackageContext> = Vec::new();
    let test_packages_results: Vec<std::result::Result<TestResults, TestResultError>> = tests.iter().map(|test|parse_test_results(student, test)).collect();
    for i in 0..test_packages_results.len() {
        match &test_packages_results[i] {
            Ok(test_package_result) => {
                match test_package_result.group_by_classname() {
                    None => {
                        test_packages.push(TestPackageContext { test_package_name: &test_package_result.test, subpackages: Vec::new(), compile_error: true, other_error: false, not_ran: false });
                    }
                    Some(s) => {
                        let mut test_subpackage: Vec<TestSubpackageContext> = Vec::new();
                        for (k,v) in s {
                            let passing_tests: Vec<TestContext> = v.iter().filter(|t|matches!(t.msg, StatusMsg::None)).map(|t|*t).map(|c|TestContext::from_test_result(c)).collect();
                            let failing_tests: Vec<TestContext> = v.iter().filter(|t|!matches!(t.msg, StatusMsg::None)).map(|t|*t).map(|c|TestContext::from_test_result(c)).collect();
                            test_subpackage.push(TestSubpackageContext { subpackage_name: k, passing_tests, failing_tests});
                        }
                        test_packages.push(TestPackageContext {test_package_name: &test_package_result.test, subpackages: test_subpackage, compile_error: false, other_error: false, not_ran: false});
                    }
                }
            }
            Err(e) => {
                match e {
                    TestResultError::IOError(_) => {
                        test_packages.push(TestPackageContext {test_package_name: &tests[i], subpackages: Vec::new(), compile_error: false, other_error: true, not_ran: false});
                    }
                    TestResultError::TestsNotRun => {
                        test_packages.push(TestPackageContext {test_package_name: &tests[i], subpackages: Vec::new(), compile_error: false, other_error: false, not_ran: true});
                    }
                }
            }
        };
    }

    let student_root_file = create_student_index(student, &files, student_template, prev_student, next_student, &test_packages)?;
    fs::write(tmpdir.path().join("index.html"), student_root_file)?;
    for (i, file) in file_paths.iter().enumerate() {
        let code = fs::read_to_string(&file)?;
        let student_report = create_student_report_html(
            &files[i].java_path,
            code,
            &files,
            &test_packages,
            prev_student,
            student,
            next_student,
            student_template
        ).map_err(|e| {
            eprintln!("Failed to create report for {}", student);
            e
        })?;
        fs::write(file, student_report)?;
    }
    flatten_move_recursive(tmpdir.path(), student_dir, None)?;

    Ok(())
}

#[derive(Serialize)]
struct StudentIndexTemplateContext<'a> {
    student: &'a str,
    prev_student: &'a str,
    next_student: &'a str,
    files: &'a Vec<StudentTemplateFile>,
    test_contexts: &'a Vec<TestPackageContext<'a>>
}

fn create_student_index(
    student: &str,
    files: &Vec<StudentTemplateFile>,
    handlebars: &Handlebars,
    prev_student: &str,
    next_student: &str,
    test_contexts: &Vec<TestPackageContext>,
) -> Result<String> {
    handlebars.render("student_index_template", &StudentIndexTemplateContext { student, files, prev_student, next_student, test_contexts }).map_err(|e|Error::new(ErrorKind::Other, e))
}
#[derive(Serialize)]
struct StudentTemplateContext<'a> {
    file: &'a str,
    files: &'a Vec<StudentTemplateFile>,
    code: &'a str,
    all_tests: &'a Vec<TestPackageContext<'a>>
}

#[derive(Serialize)]
struct TestPackageContext<'a> {
    test_package_name: &'a str,
    subpackages: Vec<TestSubpackageContext>,
    compile_error: bool,
    other_error: bool,
    not_ran: bool
}

#[derive(Serialize)]
struct TestSubpackageContext {
    subpackage_name: String,
    passing_tests: Vec<TestContext>,
    failing_tests: Vec<TestContext>,
}

#[derive(Serialize)]
struct TestContext {
    pub name: String,
    pub classname: String,
    pub time: String,
    pub msg: String,
    pub type_: String,
}

impl TestContext {
    fn from_test_result(test_result: &TestResult) -> TestContext {
        let (msg, type_): (String, String) = match test_result.msg {
            StatusMsg::None => (String::new(), String::new()),
            StatusMsg::Error { ref message, ref type_ } => (message.as_ref().map_or(String::new(), |v|String::from(v)), type_.clone()),
            StatusMsg::Failure { ref message, ref type_ } => (message.as_ref().map_or(String::new(),|v|String::from(v)), type_.clone())
        };
        TestContext {
            name: test_result.name.clone(),
            classname: test_result.classname.clone(),
            time: format!("Seconds: {}, Milliseconds: {}", test_result.time.as_secs(), test_result.time.subsec_millis()),
            msg,
            type_,
        }
    }
}

#[derive(Serialize)]
struct StudentTemplateFile {
    java_path: String,
    html_path: String,
}

fn create_student_report_html(
    file: &str,
    code: String,
    files: &Vec<StudentTemplateFile>,
    test_contexts: &Vec<TestPackageContext>,
    prev_student: &str,
    student: &str,
    next_student: &str,
    handlebars: &Handlebars
) -> Result<String> {
    // student_template.render("student_template", &StudentTemplateContext { file, files: &files, code: &code, all_tests: None }).map_err(|e|Error::new(ErrorKind::Other, e.to_string()))
    // student_template.render("student_template", &StudentTemplateContext { file, files: &files, code: &code, all_tests: test_contexts  }).map_err(|e|Error::new(ErrorKind::Other, e.to_string()))
    Ok(String::new())
}
