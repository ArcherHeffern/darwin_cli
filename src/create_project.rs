use crate::util;
use io::prelude::*;
use std::fs::File;
use std::io::{self, copy, BufReader, BufWriter};
use std::process::{Command, Stdio};
use std::{collections::HashSet, fs, path::Path, process::exit};
use tempfile::{tempdir, tempfile};
use zip::result::ZipError;
use zip::ZipArchive;

pub fn init_darwin(
    darwin_path: &Path,
    skeleton_path: &Path,
    submission_zipfile_path: &Path,
    copy_ignore_set: &HashSet<&str>,
) -> io::Result<()> {
    if !skeleton_path.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "skeleton_path must be a directory"));
    }
    if !submission_zipfile_path.extension().is_some_and(|ext|ext=="zip") {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "submission_zipfile path is not a zipfile"));
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    if darwin_path.exists() {
        print!("A project already exists here. Overwrite? (Y/n) ");
        stdout.flush()?;
        loop {
            let mut line = String::new();
            stdin.lock().read_line(&mut line)?;

            match line.as_str() {
                "y\n" => {
                    fs::remove_dir_all(darwin_path)?;
                    break;
                }
                "n\n" => {
                    exit(0);
                }
                _ => {}
            }
            print!("... ");
            stdout.flush()?;
        }
    }
    fs::create_dir(darwin_path)?;
    fs::create_dir(darwin_path.join("submission_diffs"))?;
    fs::create_dir(darwin_path.join("main"))?;
    fs::create_dir(darwin_path.join("test"))?;
    fs::create_dir(darwin_path.join("projects"))?;
    fs::create_dir(darwin_path.join("results"))?;
    File::create(darwin_path.join("tests_ran"))?;
    File::create(darwin_path.join("results").join("compile_errors"))?;
    fs::copy(
        skeleton_path.join("pom.xml"),
        darwin_path.join("main").join("pom.xml"),
    )?;

    util::copy_dir_all(
        skeleton_path.join("src").join("main"),
        darwin_path.join("main"),
        copy_ignore_set,
    )?;
    util::copy_dir_all(
        skeleton_path.join("src").join("test"),
        darwin_path.join("test"),
        copy_ignore_set,
    )?;

    submission_to_diffs(darwin_path, submission_zipfile_path, &copy_ignore_set)?;

    Ok(())
}

fn submission_to_diffs(
    darwin_path: &Path,
    submission_zipfile_path: &Path,
    file_ignore_set: &HashSet<&str>,
) -> Result<(), io::Error> {
    let file = File::open(submission_zipfile_path)?;
    let mut zip = ZipArchive::new(file)?;

    let file_names: Vec<String> = zip.file_names().map(String::from).collect();
    for file_name in file_names {
        if Path::new(&file_name)
            .extension()
            .map_or(true, |x| x != "zip")
        {
            continue;
        }

        let file_name_cpy = file_name.clone();
        let student_name = String::from(&file_name_cpy[0..file_name_cpy.find('_').unwrap()]);

        let mut student_submission_file = tempfile()?;

        if let Err(e) =
            extract_student_submission(&mut zip, file_name, &mut student_submission_file)
        {
            println!("Error extracting {}'s submission: {}", student_name, e);
            continue;
        }

        let mut student_project_zip = ZipArchive::new(student_submission_file)?;

        let src_main_dir = tempdir()?;
        if let Err(e) = extract_student_src_main(
            &mut student_project_zip,
            &src_main_dir.path(),
            file_ignore_set,
        ) {
            println!("Error extracting {}'s src/main: {}", &student_name, e);
            continue;
        }

        if let Err(e) = extract_student_pom(&mut student_project_zip, src_main_dir.path()) {
            println!("Error extracting {}'s pom.xml: {}", &student_name, e);
            continue;
        }

        let mut output = Command::new("diff")
            .arg("-ruN")
            .arg(darwin_path.join("main").to_str().unwrap())
            .arg(src_main_dir.path())
            .stdout(Stdio::piped())
            .spawn()?;

        let diff_file = match File::create(
            darwin_path
                .join("submission_diffs")
                .join(student_name.clone()),
        ) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Failed to create file '{}': {}", student_name, e);
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to create file",
                ));
            }
        };

        let mut stdout_writer = BufWriter::new(diff_file);

        if let Some(ref mut stdout) = output.stdout {
            if let Err(e) = copy(stdout, &mut stdout_writer) {
                eprintln!("Failed to write to '{}': {}", student_name, e);
                return Err(e);
            }
        } else {
            eprintln!("No stdout to write for '{}'.", student_name);
            return Err(io::Error::new(io::ErrorKind::Other, "No stdout to write"));
        }

        // Ensure the buffer is flushed after writing
        if let Err(e) = stdout_writer.flush() {
            eprintln!("Failed to flush write buffer for '{}': {}", student_name, e);
            return Err(e);
        }
    }

    Ok(())
}

fn extract_student_submission(
    zip: &mut ZipArchive<File>,
    file_name: String,
    dest: &mut File, // Changed to mutable reference
) -> Result<(), io::Error> {
    let mut file_in_zip = zip.by_name(&file_name)?;
    let mut writer = BufWriter::new(dest);
    io::copy(&mut file_in_zip, &mut writer)?;
    writer.flush()?; // Ensure all data is written to the underlying file
    Ok(())
}

fn extract_student_src_main(
    student_project_zip: &mut ZipArchive<File>,
    dest_path: &Path,
    file_ignore_set: &HashSet<&str>,
) -> Result<(), ZipError> {
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
) -> Result<(), ZipError> {
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
