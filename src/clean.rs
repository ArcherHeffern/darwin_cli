use std::{
    fs::{create_dir, remove_dir_all, OpenOptions},
    io,
};

use crate::{config::{compile_errors_file, projects_dir, results_dir}, darwin_config::{read_config, write_config}};

pub fn clean() -> io::Result<()> {
    remove_dir_all(projects_dir())?;
    create_dir(projects_dir())?;
    remove_dir_all(results_dir())?;
    create_dir(results_dir())?;
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(compile_errors_file())?;
    let mut config = read_config()?;
    config.tests_run.clear();
    write_config(config)?;
    Ok(())
}
