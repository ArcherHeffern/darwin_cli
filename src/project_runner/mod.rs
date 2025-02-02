use std::{
    collections::{HashMap, HashSet},
    fs::{self, create_dir_all, remove_file, File},
    io::{Error, ErrorKind, Result},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use zip::ZipArchive;

use crate::{
    config::{darwin_root, diff_exclude_dir, skel_dir}, darwin_config::ProjectType, types::{TestResult, TestResultError}, util::{self, create_diff, extract_zipfile, patch, path_remove_trailing_slash, project_root_in_zip}
};

mod maven;

#[derive(Clone)]
pub struct Project {
    pub project_type: ProjectType,

    /// Maps directories and files in project skeleton, to location they should be stored to when diffing and running tests
    skel_mapping: HashMap<PathBuf, PathBuf>,

    /// Maps directories and files in student submission, to location they should be stored to when diffing and running tests
    submission_zipfile_mapping: HashMap<PathBuf, PathBuf>,

    /// Not used
    _ignore: HashSet<String>,

    /// Paths that are not stored while diffing. This is calculated as skel_mapping - submission_zipfile_mapping
    /// This is useful for entries that should not be modified by students, for example testfiles.
    pub diff_exclude: HashSet<PathBuf>,

    /// Given a normalized skeleton (access via `skeleton_dir()`), lists all test names to be used as input for 
    /// * run_test_fn
    /// * relocate_test_results_fn
    /// * parse_result_report_fn
    list_tests_fn: fn(&Project) -> HashSet<String>,

    /// Given a normalized project (`&Path`), compiles the project
    compile_fn: fn(&Project, &Path) -> Result<()>,

    /// Given a normalized project (`&Path`), and a test name (`&str`), run tests and produce a test report
    run_test_fn: fn(&Project, &Path, &str) -> Result<()>,

    /// May remove
    relocate_test_results_fn: fn(&Project, &Path, &str, &Path) -> Result<()>, // project, project_path, test, dest_file

    /// Given a test report (&Path), parse into `Vec<TestResult>`
    parse_result_report_fn:
        fn(&Project, &Path, &str, &str) -> std::result::Result<Vec<TestResult>, TestResultError>,
}

pub fn project_type_to_project(project_type: &ProjectType) -> Result<Project> {
    match project_type {
        ProjectType::None => no_project(),
        ProjectType::MavenSurefire => maven_project(), 
        ProjectType::Go => go_project(),
    }
}

pub fn no_project() -> Result<Project> {
    maven_project()
}

pub fn maven_project() -> Result<Project> {
    let mut skel_mapping = HashMap::new();
    skel_mapping.insert(PathBuf::from("src/main/"), PathBuf::from("src/main/"));
    skel_mapping.insert(PathBuf::from("src/test/"), PathBuf::from("src/test/"));
    skel_mapping.insert(PathBuf::from("pom.xml"), PathBuf::from("pom.xml"));

    let mut submission_zipfile_mapping = HashMap::new();
    submission_zipfile_mapping.insert(PathBuf::from("pom.xml"), PathBuf::from("pom.xml"));
    submission_zipfile_mapping.insert(PathBuf::from("src/main/"), PathBuf::from("src/main/"));

    let mut ignore = HashSet::new();
    ignore.insert(String::from(".DS_Store"));
    ignore.insert(String::from("doc"));
    ignore.insert(String::from(".settings"));
    ignore.insert(String::from(".project"));
    ignore.insert(String::from(".classpath"));
    ignore.insert(String::from(".git"));
    ignore.insert(String::from(".gitignore"));

    Project::new(
        ProjectType::MavenSurefire, 
        skel_mapping,
        submission_zipfile_mapping,
        ignore,
        maven::compile,
        maven::list_tests,
        maven::run_test,
        maven::relocate_test_results,
        maven::parse_result_report,
    )
}

pub fn go_project() -> Result<Project> {
    todo!();
}

impl Project {
    /// skel_mapping may not map 2 src paths to the same dest path
    pub fn new(
        project_type: ProjectType,
        skel_mapping: HashMap<PathBuf, PathBuf>,
        submission_zipfile_mapping: HashMap<PathBuf, PathBuf>,
        _ignore: HashSet<String>,
        compile_fn: fn(&Project, &Path) -> Result<()>,
        list_tests_fn: fn(&Project) -> HashSet<String>,
        run_test_fn: fn(&Project, &Path, &str) -> Result<()>,
        relocate_test_results_fn: fn(&Project, &Path, &str, &Path) -> Result<()>, // project, project_path, test, dest_file
        parse_result_report_fn: fn(
            &Project,
            &Path,
            &str,
            &str,
        )
            -> std::result::Result<Vec<TestResult>, TestResultError>,
    ) -> Result<Self> {
        if skel_mapping.values().collect::<HashSet<&PathBuf>>().len() != skel_mapping.values().len() {
            return Err(Error::new(ErrorKind::Other, "skel_mapping cannot map multiple source directories to the same dest directory"));
        }
        let skel_destinations: HashSet<PathBuf> = skel_mapping.values().cloned().collect();
        let submission_destinations: HashSet<PathBuf> =
            submission_zipfile_mapping.values().cloned().collect();
        let diff_exclude = skel_destinations
            .iter()
            .cloned()
            .filter(|skel_dest| !submission_destinations.contains(skel_dest))
            .collect();

        Ok(Project {
            project_type,
            skel_mapping,
            submission_zipfile_mapping,
            _ignore,
            diff_exclude,
            compile_fn,
            list_tests_fn,
            run_test_fn,
            relocate_test_results_fn,
            parse_result_report_fn,
        })
    }

    pub fn init_skeleton(&self, skeleton_path: &Path) -> Result<()> {
        for (from, to) in self.skel_mapping.iter() {
            if !skeleton_path.join(from).exists() {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("Expected {:?} to exist in skeleton", from),
                ));
            }

            let from = skeleton_path.join(from);
            let to = match self.diff_exclude.contains(to) {
                true => {
                    diff_exclude_dir()
                }
                false => skel_dir(),
            }
            .join(to);
            if !from.to_string_lossy().ends_with('/') {
                if !from.is_file() {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Expected {:?} to be a file", &from),
                    ));
                } else {
                    if let Some(parent) = to.parent() {
                        create_dir_all(parent)?;
                    }
                    fs::copy(&from, &to).map_err(|e| {
                        Error::new(
                            ErrorKind::Other,
                            format!("Failed to copy {:?} to {:?}: {}", from, to, e),
                        )
                    })?;
                }
            }
            if from.to_string_lossy().ends_with('/') {
                if !from.is_dir() {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Expected {:?} to be a directory", from),
                    ));
                } else {
                    util::copy_dir_all(&from, &to, None).map_err(|e| {
                        Error::new(
                            ErrorKind::Other,
                            format!("Failed to copy {:?} to {:?}: {}", from, to, e),
                        )
                    })?;
                }
            }
        }
        Ok(())
    }

    pub fn zip_submission_to_normalized_form(
        &self,
        zip: &mut ZipArchive<File>,
        dest_dir: &Path,
        copy_ignore_set: Option<&HashSet<&str>>,
    ) -> Result<()> {
        if zip.is_empty() {
            return Err(Error::new(
                ErrorKind::NotFound,
                "Expected zip to have contents",
            ));
        }

        let root = project_root_in_zip(zip, &self.submission_zipfile_mapping.keys().collect())?;

        let files_to_extract: HashMap<usize, PathBuf> = (0..zip.len()) // Index -> Dest
            .filter_map(|i| {
                let file = zip.by_index(i).ok()?;
                let filename = PathBuf::from(file.name());
                let file_name = filename.file_name()?.to_str()?;
                if copy_ignore_set.is_some_and(|s| s.contains(file_name)) {
                    return None;
                }
                let filename = PathBuf::from(filename.strip_prefix(&root).ok()?);

                for (k, v) in self.submission_zipfile_mapping.iter() {
                    if filename.starts_with(k) {
                        if filename.ends_with(Path::new("/")) {
                            let dest = v.join(filename.strip_prefix(k).ok()?);
                            return Some((i, dest));
                        } else {
                            return Some((i, filename));
                        }
                    }
                }
                None
            })
            .collect();

        for (i, dest) in files_to_extract {
            // Write this remapping into the config file
            let zipfile = zip.by_index(i).unwrap();
            extract_zipfile(zipfile, &dest_dir.join(&dest))?;
        }

        Ok(())
    }

    /// Symlink all entries of self.diff_exclude in
    /// Invariants:
    /// - darwin_path is an existing .darwin project root directory
    /// - dest does not exist
    pub fn recreate_normalized_project(&self, dest: &Path, diff_path: &Path) -> Result<()> {
        assert!(darwin_root().is_dir());
        assert!(!dest.exists());

        patch(&skel_dir(), diff_path, dest, true)?;

        for excluded in self.diff_exclude.iter() {
            let original = diff_exclude_dir().join(excluded).canonicalize()?;
            let mut link = dest.join(excluded);
            if let Some(parent) = link.parent() {
                fs::create_dir_all(parent)?;
            }
            link = path_remove_trailing_slash(&link);
            symlink(&original, &link).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to symlink {:?} to {:?}: {}", link, original, e),
                )
            })?;
        }

        Ok(())
    }

    /// Recreates students original project structure`
    /// May later implement symlinking all diff_exclude files back in
    /// symlink field for now does nothing
    /// 
    /// Invariants:
    /// - project_path is an existing .darwin project root directory
    /// - dest is an empty directory
    pub fn recreate_original_project(&self, normalized_project: &Path, symlink: bool) -> Result<()> {
        todo!();
    }

    pub fn compile(&self, project_path: &Path) -> Result<()> {
        (self.compile_fn)(self, project_path)
    }

    /// This should only be used when creating a project
    pub fn list_tests(&self) -> HashSet<String> {
        if let Err(e) = self.denormalize_skel() {
            eprintln!("{}", e);
        }
        let mut res = HashSet::new();
        match self.normalize_skel() {
            Ok(()) => {
                res = (self.list_tests_fn)(self);
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }
        if let Err(e) = self.denormalize_skel() {
            eprintln!("{}", e);
        }
        res
    }

    /// Add all symlinks to the skeleton
    fn normalize_skel(&self) -> Result<()> {
        for excluded in self.diff_exclude.iter() {
            let original = diff_exclude_dir().join(excluded).canonicalize()?;
            let mut link = skel_dir().join(excluded);
            link = path_remove_trailing_slash(&link);
            symlink(&original, &link).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to symlink {:?} to {:?}: {}", link, original, e),
                )
            })?;
        }

        Ok(())
    }

    /// Remove all symlinks from the skeleton. Should only be used after `normalize_skel`
    fn denormalize_skel(&self) -> Result<()> {
        for excluded in self.diff_exclude.iter() {
            let remove = skel_dir().join(excluded);
            let remove = path_remove_trailing_slash(&remove);
            if !remove.exists() {
                continue;
            }
            remove_file(&remove).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to remove what should be a symlink {:?}: {}", &remove, e),
                )
            })?;
        }

        Ok(())

    }

    pub fn run_test(&self, project_path: &Path, test: &str) -> Result<()> {
        (self.run_test_fn)(self, project_path, test)
    }

    pub fn relocate_test_results(
        &self,
        project_path: &Path,
        test: &str,
        dest_file: &Path,
    ) -> Result<()> {
        (self.relocate_test_results_fn)(self, project_path, test, dest_file)
    }

    pub fn parse_result_report(
        &self,
        report_path: &Path,
        student: &str,
        test: &str,
    ) -> std::result::Result<Vec<TestResult>, TestResultError> {
        (self.parse_result_report_fn)(self, report_path, student, test)
    }
}


pub mod test {
    use std::fs::{remove_file, File};
    use std::{fs::remove_dir_all, path::Path};

    use assert_fs::fixture::ChildPath;
    use zip::ZipArchive;

    use crate::config::darwin_root;
    use crate::util::create_diff;

    use super::maven_project;
    use assert_fs::{self, assert::PathAssert, prelude::{FileTouch, FileWriteStr, PathChild}};
    use predicates::prelude::*;

    #[test]
    fn test_init_skeleton() {
        if darwin_root().exists() {
            remove_dir_all(darwin_root()).unwrap();
        }
        let project = maven_project().unwrap();
        project.init_skeleton(Path::new("./testing/Skel")).unwrap();
    }

    #[test]
    fn test_zip_submission_to_normalized_form() {
        let temp_dir = assert_fs::TempDir::new().unwrap();

        let file = File::open("./testing/Impl.zip").unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        let project = maven_project().unwrap();
        project
            .zip_submission_to_normalized_form(&mut zip, temp_dir.path(), None)
            .unwrap();

        temp_dir.child("src").assert(predicate::path::exists());
        temp_dir.close().unwrap();
    }

    #[test]
    fn test_create_normalized_project_diff() {
        let normalized_project_dest = assert_fs::TempDir::new().unwrap();
        let diff_dest_dir = assert_fs::TempDir::new().unwrap();
        let diff_dest = diff_dest_dir.child("tmp");
        let src = File::open("./testing/Impl.zip").unwrap();
        let mut zip = ZipArchive::new(src).unwrap();

        let project = maven_project().unwrap();
        project
            .zip_submission_to_normalized_form(&mut zip, &normalized_project_dest, None)
            .unwrap();
        normalized_project_dest.child("src").assert(predicate::path::exists());
        create_diff(&Path::new("./testing/Skel"), &normalized_project_dest, &diff_dest)
            .unwrap();

        normalized_project_dest.close().unwrap();
        diff_dest_dir.close().unwrap();
    }

    fn test_recreate_original_project() {

    }
}
