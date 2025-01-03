use std::{
    fs::{self, create_dir, remove_dir_all},
    io::{Error, ErrorKind, Result},
    path::Path},
};

use serde::Serialize;
use tempfile::tempdir;
use tinytemplate::TinyTemplate;

use crate::{
    config::{darwin_root, student_diff_file, tests_ran_file},
    list_students::list_students,
    list_tests::list_tests,
    util::{
        file_contains_line, flatten_move_recursive, list_files_recursively, recreate_student_main,
    },
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

    create_report_student_list(&report_root.join("index.html"), &students)?;
    create_student_reports(report_root, tests, &students)?;

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
    
    fs::write(report_root.join("styles").join("index.css"), include_str!("../template/index.css"))?;
    fs::write(report_root.join("styles").join("student_index.css"), include_str!("../template/student_index.css"))?;
    fs::write(report_root.join("styles").join("student.css"), include_str!("../template/student.css"))?;
    fs::write(report_root.join("styles").join("LibreBaskerville-Regular.ttf"), include_bytes!("../template/LibreBaskerville-Regular.ttf"))?;
    fs::write(report_root.join("styles").join("LibreBaskerville-Italic.ttf"), include_bytes!("../template/LibreBaskerville-Italic.ttf"))?;
    fs::write(report_root.join("styles").join("LibreBaskerville-Bold.ttf"), include_bytes!("../template/LibreBaskerville-Bold.ttf"))?;
    fs::write(report_root.join("styles").join("OFL.txt"), include_bytes!("../template/OFL.txt"))?;
    Ok(())
}

fn create_report_student_list(dest: &Path, students: &[String]) -> Result<()> {
    let mut tt = TinyTemplate::new();
    tt.add_template("student_list", include_str!("../template/index.html"))
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

    let rendered = tt
        .render("student_list", &StudentListContext { students })
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

    fs::write(dest, rendered.as_bytes())
}

fn create_student_reports(
    report_root: &Path,
    tests: &Vec<String>,
    students: &[String]
) -> Result<()> {
    let mut student_template = TinyTemplate::new();
    student_template.add_template("student_template", include_str!("../template/student.html"))
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    student_template.add_template("student_index_template", include_str!("../template/student_index.html"))
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

    let mut prev_student = "";
    for i in 0..students.len() - 1 {
        let student = students[i].as_str();
        create_student_report(
            report_root,
            tests,
            &prev_student,
            &student,
            &students[i + 1],
            &student_template
        )?;
        prev_student = student;
    }
    create_student_report(
        report_root,
        tests,
        &prev_student,
        &students[students.len() - 1],
        "",
        &student_template
    )?;
    Ok(())
}

fn create_student_report(
    report_root: &Path,
    tests: &Vec<String>,
    prev_student: &str,
    student: &str,
    next_student: &str,
    student_template: &TinyTemplate<'_>
) -> Result<()> {
    _create_student_report(report_root, tests, prev_student, student, next_student, student_template)
}

fn _create_student_report(
    report_root: &Path,
    tests: &Vec<String>,
    prev_student: &str,
    student: &str,
    next_student: &str,
    student_template: &TinyTemplate<'_>
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

    let student_root_file = create_student_index(student, &files, student_template, prev_student, next_student)?;
    fs::write(tmpdir.path().join("index.html"), student_root_file)?;
    for (i, file) in file_paths.iter().enumerate() {
        let code = fs::read_to_string(&file)?;
        let student_report = create_student_report_html(
            &files[i].java_path,
            code,
            &files,
            tests,
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
}

fn create_student_index(
    student: &str,
    files: &Vec<StudentTemplateFile>,
    student_template: &TinyTemplate<'_>,
    prev_student: &str,
    next_student: &str
) -> Result<String> {
    student_template.render("student_index_template", &StudentIndexTemplateContext { student, files, prev_student, next_student }).map_err(|e|Error::new(ErrorKind::Other, e))
}
#[derive(Serialize)]
struct StudentTemplateContext<'a> {
    file: &'a str,
    files: &'a Vec<StudentTemplateFile>,
    code: &'a str
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
    tests: &Vec<String>,
    prev_student: &str,
    student: &str,
    next_student: &str,
    student_template: &TinyTemplate<'_>
) -> Result<String> {
    student_template.render("student_template", &StudentTemplateContext { file, files: &files, code: &code }).map_err(|e|Error::new(ErrorKind::Other, e.to_string()))
}
