use std::{
    fs::{create_dir, remove_dir_all, File, OpenOptions},
    io,
    path::Path,
};

pub fn clean(darwin_path: &Path) -> io::Result<()> {
    remove_dir_all(darwin_path.join("projects"))?;
    create_dir(darwin_path.join("projects"))?;
    remove_dir_all(darwin_path.join("results"))?;
    create_dir(darwin_path.join("results"))?;
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(darwin_path.join("results").join("compile_errors"))?;
    File::create(darwin_path.join("tests_ran"))?;
    Ok(())
}
