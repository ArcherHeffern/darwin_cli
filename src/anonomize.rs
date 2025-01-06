use std::{collections::HashSet, fs::rename, io::Result};

use crate::{
    config::{compile_errors_file, student_diff_file, student_result_file},
    list_students::list_students,
    list_tests::list_tests,
    util::file_replace_line,
};

pub fn anonomize() {
    // Currently does not do any renaming within diffs
    _anonomize();
}

fn _anonomize() {
    let tests = list_tests();

    for (i, student) in list_students().iter().enumerate() {
        if anonomize_student(student, i, &tests).is_err() {
            eprintln!("Failed to anonomize {}", student);
        }
    }
}

fn anonomize_student(student: &str, i: usize, tests: &HashSet<String>) -> Result<()> {
    for test in tests {
        if student_result_file(student, test).is_file() {
            rename(
                student_result_file(student, test),
                student_result_file(&i.to_string(), test),
            )?;
        }
    }
    rename(
        student_diff_file(student),
        student_diff_file(&i.to_string()),
    )?;

    let mut new_name = i.to_string();
    new_name.push('\n');
    file_replace_line(&compile_errors_file(), student, &new_name)?;

    Ok(())
}
