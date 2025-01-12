use crate::config::{
    compile_errors_file, darwin_root, diff_dir, main_dir, projects_dir, results_dir, student_diff_file, test_dir, tests_ran_file
};
use crate::util::extract_file;
use std::fs::{remove_dir_all, File};
use std::io::{Error, ErrorKind, Result};
use std::{collections::HashSet, fs, path::Path};
use tempfile::{tempdir, tempfile};
use zip::ZipArchive;
use crate::project_runner::{self, MavenProject};

pub fn create_darwin(
    project_skeleton: &Path,
    moodle_submissions_zipfile: &Path,
    copy_ignore_set: &HashSet<&str>,
) -> Result<()> {
    if darwin_root().exists() {
        return Err(Error::new(
            ErrorKind::AlreadyExists,
            "darwin project already exists in this directory",
        ));
    }
    if !project_skeleton.is_dir() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "skeleton_path must be a directory",
        ));
    }
    if !moodle_submissions_zipfile.is_file() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "moodle_submissions_zipfile path is not a zipfile",
        ));
    }
    if !moodle_submissions_zipfile
        .extension()
        .map_or(true, |ext| ext == "zip")
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "moodle_submissions_zipfile path is not a zipfile",
        ));
    }

    let status = _create_darwin(
        project_skeleton,
        moodle_submissions_zipfile,
        copy_ignore_set,
    );

    if status.is_err() && darwin_root().is_dir() {
        remove_dir_all(darwin_root())?;
    }

    status
}

fn _create_darwin(
    skeleton_path: &Path,
    submission_zipfile_path: &Path,
    copy_ignore_set: &HashSet<&str>,
) -> Result<()> {
    fs::create_dir_all(darwin_root())?;
    fs::create_dir_all(diff_dir())?;
    fs::create_dir_all(main_dir())?;
    fs::create_dir_all(test_dir())?;
    fs::create_dir_all(projects_dir())?;
    fs::create_dir_all(results_dir())?;
    File::create(tests_ran_file())?; // Possible error for this and below line if the leading paths don't exist.
    File::create(compile_errors_file())?;

    project_runner::MavenProject::new().init_skeleton(skeleton_path, Some(copy_ignore_set))?;


    submissions_to_diffs(submission_zipfile_path, copy_ignore_set, |s, e| {
        eprintln!("Error extracting {}'s submission: {}", s, e)
    })?;

    Ok(())
}

fn submissions_to_diffs(
    submission_zipfile_path: &Path,
    copy_ignore_set: &HashSet<&str>,
    on_submission_extraction_error: fn(&str, &Error), // Student name
) -> Result<()> {
    let zip = File::open(submission_zipfile_path)?;
    let mut zip = ZipArchive::new(zip)?;

    let file_names: Vec<String> = zip.file_names().map(String::from).collect();
    for file_name in file_names {
        if Path::new(&file_name)
            .extension()
            .map_or(true, |x| x != "zip")
        {
            continue;
        }

        let student_name = &file_name[0..file_name.find('_').expect("Moodle submission zipfile must delimit all contained student submission zipfiles with '_'. Perhaps moodle changed its naming scheme or this isn't a moodle submission zipfile.")];

        let mut student_submission_file = tempfile()?;

        if let Err(e) = extract_file(&mut zip, &file_name, &mut student_submission_file) {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Error extracting {}'s submission: {}", student_name, e),
            ));
        }

        let mvn = MavenProject::new();
        let mut student_project_zip = ZipArchive::new(student_submission_file)?;
        let normalized_project = tempdir()?;
        mvn.zip_submission_to_normalized_form(&mut student_project_zip, normalized_project.path(), Some(copy_ignore_set))
            .inspect_err(|e|on_submission_extraction_error(student_name, e))?;
        mvn.create_normalized_project_diff(normalized_project.path(), &student_diff_file(student_name))
            .inspect_err(|e|on_submission_extraction_error(student_name, e))?;
    }

    Ok(())
}
