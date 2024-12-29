use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{copy, prelude::*, BufReader};
use std::io::{self, BufRead, BufWriter};
use std::path::Path;
use std::process::{exit, Stdio};
use std::process::Command;
use tempfile::{tempdir, tempfile};
use util::utils::{self, copy_dir_all};
use zip::result::ZipError;
use zip::ZipArchive;

pub mod util;

struct TestResult {
    correct: i32
}

fn main() {
    let skeleton_path = Path::new("./skel");
    let submission_zipfile_path = Path::new("./243COSI-131A-1-PA1-59260.zip");
    let project_path: &Path = Path::new(".darwin");

    let mut copy_ignore_set = HashSet::new();
    copy_ignore_set.insert(".DS_Store");


    init_darwin(project_path, skeleton_path, submission_zipfile_path, &copy_ignore_set).unwrap();

    // println!("Tests: {:?}", list_tests(project_path));

    set_active_project(project_path, Path::new("./.darwin/submission_diffs/<student.diff"), &copy_ignore_set).unwrap();

    // let test = "WorkingDirectoryTests.java";

    // run_test(project_path, test).unwrap();
}

fn init_darwin(project_path: &Path, skeleton_path: &Path, submission_zipfile_path: &Path, copy_ignore_set: &HashSet<&str>) -> Result<(), io::Error> {
    assert!(skeleton_path.is_dir());
    assert!(submission_zipfile_path.extension().unwrap() == "zip");

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
        project_path.join("main").join("pom.xml"),
    )?;

    util::utils::copy_dir_all(
        skeleton_path.join("src").join("main"),
        project_path.join("main"),
        &copy_ignore_set
    )?;
    util::utils::copy_dir_all(
        skeleton_path.join("src").join("test"),
        project_path.join("project").join("src").join("test"),
        &copy_ignore_set
    )?;

    submission_to_diffs(project_path, submission_zipfile_path, &copy_ignore_set)?;

    Ok(())
}

fn list_tests(project_path: &Path) -> Vec<String> {
    let test_dir = project_path.join("project").join("src").join("test");
    let files = utils::list_files_recursively(&test_dir);

    let mut out = Vec::new();
    for file in files {
        out.push(String::from(file.file_name().unwrap().to_str().unwrap()));
    }

    out
}

fn set_active_project(project_path: &Path, diff_path: &Path, copy_ignore_set: &HashSet<&str>) -> Result<(), io::Error> {
    // diff_path: Contains the full project relative path
    // rm -rf .darwin/project/src/main
    // cp -r .darwin/main/ .darwin/project/src/main
    // patch -d .darwin/project/src/main -p2 < .darwin/submission_diffs/<student_diff
    // mv .darwin/project/src/main/pom.xml .darwin/project/pom
    let project_main_path = project_path.join("project").join("src").join("main");
    if project_main_path.exists() {
        if let Err(e) = fs::remove_dir_all(&project_main_path) {
            eprintln!("Error removing {} when setting active project: {}", project_main_path.to_str().unwrap(), e);
            return Err(e);
        }
    }
    copy_dir_all(project_path.join("main"), &project_main_path, copy_ignore_set).unwrap();

    let mut output = Command::new("patch")
        .arg("-d")
        .arg(&project_main_path)
        .arg("-p2")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()?;

    match output.stdin.take() {
        Some(stdin) => {
            let mut stdin_writer = BufWriter::new(stdin);
            let patch_file = File::open(diff_path)?;
            let mut patch_reader = BufReader::new(patch_file);
            copy(&mut patch_reader, &mut stdin_writer)?;
        },
        None => {
            eprintln!("Cannot access stdin of patch process {}", output.id());
        }
    }

    let status = output.wait()?;
    if !status.success() {
        eprintln!("Patch command failed with status: {}", status);
    }

    fs::rename(project_path.join("project").join("src").join("main").join("pom.xml"), project_path.join("project").join("pom.xml"))?;

    Ok(())
}

fn run_test(project_path: &Path, test: &str) -> Result<TestResult, io::Error> {
    // Assume the student diff has already been resolved and placed into .darwin/project/src/main
    Ok(TestResult { correct: 1 })
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

        if let Err(e) = extract_student_pom(&mut student_project_zip, &src_main_dir.path()) {
            println!("Error extracting {}'s pom.xml: {}", &student_name, e);
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

fn extract_student_pom(student_project_zip: &mut ZipArchive<File>, dest_dir: &Path) -> Result<(), ZipError> {
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
            writer.flush()
                .map_err(|e| ZipError::Io(io::Error::new(io::ErrorKind::Other, e)))?;

            // Since pom.xml was found and copied, we break out of the loop
            break;
        }
    }

    Ok(())
}