use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{copy, prelude::*};
use std::io::{self, BufRead, BufWriter};
use std::path::Path;
use std::process::{exit, Stdio};
use std::process::Command;
use tempfile::{tempdir, tempfile};
use zip::result::ZipError;
use zip::ZipArchive;

pub mod util;


fn main() {
    let skeleton_path = Path::new("./skel");
    let submission_zipfile_path = Path::new("./243COSI-131A-1-PA1-59260.zip");

    init_darwin(skeleton_path, submission_zipfile_path).unwrap();
}

fn init_darwin(skeleton_path: &Path, submission_zipfile_path: &Path) -> Result<(), io::Error> {
    assert!(skeleton_path.is_dir());
    assert!(submission_zipfile_path.extension().unwrap() == "zip");

    let project_path: &Path = Path::new(".darwin");
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    if project_path.exists() {
        print!("A project already exists here. Overwrite? (Y/n) ");
        stdout.flush()?;
        loop {
            let mut line = String::new();
            stdin.lock().read_line(&mut line)?;

            match line.as_str() {
                "y\n" => {
                    fs::remove_dir_all(project_path)?;
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
    fs::create_dir(project_path)?;
    fs::create_dir(project_path.join("project"))?;
    fs::create_dir(project_path.join("project").join("src"))?;
    fs::create_dir(project_path.join("submission_diffs"))?;
    fs::create_dir(project_path.join("main"))?;
    fs::copy(
        skeleton_path.join("pom.xml"),
        project_path.join("project").join("pom.xml"),
    )?;

    let mut file_ignore_set = HashSet::new();
    file_ignore_set.insert(".DS_Store");

    util::utils::copy_dir_all(
        skeleton_path.join("src").join("main"),
        project_path.join("main"),
        &file_ignore_set
    )?;
    util::utils::copy_dir_all(
        skeleton_path.join("src").join("test"),
        project_path.join("project").join("src").join("test"),
        &file_ignore_set
    )?;

    submission_to_diffs(project_path, submission_zipfile_path, &file_ignore_set)?;

    Ok(())
}

fn submission_to_diffs(project_path: &Path, submission_zipfile_path: &Path, file_ignore_set: &HashSet<&str>) -> Result<(), io::Error> {
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

        if let Err(e) = extract_student_submission(&mut zip, file_name, &mut student_submission_file) {
            println!("Error extracting {}'s submission: {}", student_name, e);
            continue;
        }

        let mut student_project_zip = ZipArchive::new(student_submission_file)?;

        let src_main_dir = tempdir()?;
        if let Err(e) = extract_student_src_main(&mut student_project_zip, &src_main_dir.path(), file_ignore_set) {
            println!("Error extracting {}'s src/main: {}", &student_name, e);
            continue;
        }

        let mut output = Command::new("diff")
            .arg("-ru")
            .arg(project_path.join("main").to_str().unwrap())
            .arg(src_main_dir.path())
            .stdout(Stdio::piped())
            .spawn()?;


        let diff_file = match File::create(project_path.join("submission_diffs").join(student_name.clone())) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Failed to create file '{}': {}", student_name, e);
                return Err(io::Error::new(io::ErrorKind::Other, "Failed to create file"));
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
    dest: &mut File,  // Changed to mutable reference
) -> Result<(), io::Error> {
    let mut file_in_zip = zip.by_name(&file_name)?;
    let mut writer = BufWriter::new(dest);
    io::copy(&mut file_in_zip, &mut writer)?;
    writer.flush()?;  // Ensure all data is written to the underlying file
    Ok(())
}

fn extract_student_src_main(
    student_project_zip: &mut ZipArchive<File>,
    dest_path: &Path,
    file_ignore_set: &HashSet<&str>
) -> Result<(), ZipError> {
    let tmp = Path::new("src").join("main");
    let main_path = tmp.to_str().unwrap();

    util::utils::extract_directory_from_zip(
        student_project_zip,
        dest_path.to_str().unwrap(),
        main_path,
        file_ignore_set
    )
}
