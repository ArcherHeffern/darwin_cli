use crate::config::{
    compile_errors_file, darwin_root, diff_dir, projects_dir, results_dir, skel_dir, student_diff_file
};
use crate::darwin_config::{write_config, DarwinConfig};
use crate::util::{create_diff, extract_file};
use std::collections::HashMap;
use std::fs::{remove_dir_all, File};
use std::io::{Error, ErrorKind, Result};
use std::{collections::HashSet, fs, path::Path};
use tempfile::{tempdir, tempfile};
use zip::ZipArchive;
use crate::project_runner::Project;

pub fn create_darwin(
    project: &Project,
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
        project,
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
    project: &Project,
    skeleton_path: &Path,
    submission_zipfile_path: &Path,
    copy_ignore_set: &HashSet<&str>,
) -> Result<()> {
    fs::create_dir_all(darwin_root())?;
    fs::create_dir_all(diff_dir())?;
    fs::create_dir_all(projects_dir())?;
    fs::create_dir_all(results_dir())?;
    File::create(compile_errors_file())?;

    project.init_skeleton(skeleton_path)?;


    let mut extraction_errors: HashMap<String, String> = HashMap::new();

    submissions_to_diffs(project, submission_zipfile_path, copy_ignore_set, &mut |s, e| {
        eprintln!("Error parsing {}'s submission: {}", s, e);
        extraction_errors.insert(s.to_string(), e.to_string());
    })?;

    create_config(project, extraction_errors)?;
    Ok(())
}

fn create_config(project: &Project, extraction_errors: HashMap<String, String>) -> Result<()> {
    // Expensive list tests
    let tests: Vec<String> = project.list_tests().iter().cloned().collect();
    let config = DarwinConfig { version: String::from("1.0.0"), project_type: project.project_type.clone(), tests, tests_run: Vec::new(), extraction_errors };
    write_config(config)?;
    Ok(())
}

fn submissions_to_diffs<F>(
    project: &Project,
    submission_zipfile_path: &Path,
    copy_ignore_set: &HashSet<&str>,
    on_submission_extraction_error: &mut F, // Student name
) -> Result<()> 
where F: for<'a> FnMut(&'a str, &'a std::io::Error)
{
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

        let mut student_project_zip = ZipArchive::new(student_submission_file)?;
        let normalized_project = tempdir()?;
        if let Err(e) = project.zip_submission_to_normalized_form(&mut student_project_zip, normalized_project.path(), Some(copy_ignore_set)) {
            on_submission_extraction_error(student_name, &e);
            continue;
        }
        if let Err(e) = create_diff(&skel_dir(), normalized_project.path(), &student_diff_file(student_name)) {
            on_submission_extraction_error(student_name, &e);
            continue;
        }
    }

    Ok(())
}
