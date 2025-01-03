use crate::config::{compile_errors_file, darwin_root, diff_dir, main_dir, projects_dir, results_dir, student_diff_file, test_dir, tests_ran_file};
use crate::util::{self, create_diff};
use io::prelude::*;
use std::fs::{remove_dir_all, File};
use std::io::{self, copy, BufReader, BufWriter, Error, ErrorKind, Result};
use std::{collections::HashSet, fs, path::Path};
use tempfile::{tempdir, tempfile};
use zip::result::ZipError;
use zip::ZipArchive;

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
    fs::copy(
        skeleton_path.join("pom.xml"),
        main_dir().join("pom.xml"),
    )?;

    util::copy_dir_all(
        skeleton_path.join("src").join("main"),
        main_dir(),
        copy_ignore_set,
    )?;
    util::copy_dir_all(
        skeleton_path.join("src").join("test"),
        test_dir(),
        copy_ignore_set,
    )?;

    submissions_to_diffs(
        submission_zipfile_path,
        copy_ignore_set,
        |s, e| eprintln!("Error extracting {}'s submission: {}", s, e),
    )?;

    Ok(())
}

fn submissions_to_diffs(
    submission_zipfile_path: &Path,
    file_ignore_set: &HashSet<&str>,
    on_submission_extraction_error: fn(&str, Error), // Student name
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

        if let Err(e) = submission_to_diff(
            &file_name,
            &mut zip,
            student_name,
            file_ignore_set,
        ) {
            on_submission_extraction_error(student_name, e);
        }
    }

    Ok(())
}

fn submission_to_diff(
    file_name: &str,
    zip: &mut ZipArchive<File>,
    student_name: &str,
    file_ignore_set: &HashSet<&str>,
) -> Result<()> {
    let mut student_submission_file = tempfile()?;

    if let Err(e) = extract_student_submission(zip, file_name, &mut student_submission_file) {
        return Err(Error::new(
            ErrorKind::Other,
            format!("Error extracting {}'s submission: {}", student_name, e),
        ));
    }

    let mut student_project_zip = ZipArchive::new(student_submission_file)?;

    let src_main_dir = tempdir()?;
    if let Err(e) = extract_student_src_main(
        &mut student_project_zip,
        src_main_dir.path(),
        file_ignore_set,
    ) {
        return Err(Error::new(
            ErrorKind::Other,
            format!("Error extracting {}'s submission: {}", student_name, e),
        ));
    }

    if let Err(e) = extract_student_pom(&mut student_project_zip, src_main_dir.path()) {
        return Err(Error::new(
            ErrorKind::Other,
            format!("Error extracting {}'s pom.xml: {}", &student_name, e),
        ));
    }

    let deviant = src_main_dir.path();
    create_diff(&main_dir(), deviant, &student_diff_file(student_name))
}

fn extract_student_submission(
    zip: &mut ZipArchive<File>,
    file_name: &str,
    dest: &mut File,
) -> Result<()> {
    let mut file_in_zip = zip.by_name(file_name)?;
    let mut writer = BufWriter::new(dest);
    io::copy(&mut file_in_zip, &mut writer)?;
    writer.flush()?; // Ensure all data is written to the underlying file
    Ok(())
}

fn extract_student_src_main(
    student_project_zip: &mut ZipArchive<File>,
    dest_path: &Path,
    file_ignore_set: &HashSet<&str>,
) -> std::result::Result<(), ZipError> {
    let tmp = Path::new("src").join("main");
    let main_path = tmp.to_str().unwrap();

    util::extract_directory_from_zip(
        student_project_zip,
        dest_path.to_str().unwrap(),
        main_path,
        file_ignore_set,
    )
}

fn extract_student_pom(
    student_project_zip: &mut ZipArchive<File>,
    dest_dir: &Path,
) -> std::result::Result<(), ZipError> {
    for i in 0..student_project_zip.len() {
        let file = student_project_zip.by_index(i)?;

        // Check if the file name contains 'pom.xml'
        if file.name().contains("pom.xml") {
            // Create a file in the destination directory to write to
            let dest_path = dest_dir.join("pom.xml");
            let dest_file = File::create(&dest_path)
                .map_err(|e| ZipError::Io(io::Error::new(io::ErrorKind::Other, e)))?;

            // Set up the reader and writer
            let mut reader = BufReader::new(file);
            let mut writer = BufWriter::new(dest_file);

            // Copy the contents of the zip entry to the new file
            copy(&mut reader, &mut writer)
                .map_err(|e| ZipError::Io(io::Error::new(io::ErrorKind::Other, e)))?;

            // Flush the writer to ensure all data is written
            writer
                .flush()
                .map_err(|e| ZipError::Io(io::Error::new(io::ErrorKind::Other, e)))?;

            // Since pom.xml was found and copied, we break out of the loop
            break;
        }
    }

    Ok(())
}
