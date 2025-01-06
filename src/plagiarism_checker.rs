use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Error, ErrorKind, Result, Write};
use std::path::Path;

use handlebars::Handlebars;
use serde::Serialize;
use tlsh_fixed::{
    BucketKind::Bucket128, ChecksumKind::OneByte, Tlsh, TlshBuilder, Version::Version4,
};

use crate::config::student_diff_file;
use crate::list_students::list_students;
use crate::util::{buffer_flatmap, is_student};
use pcoa::apply_pcoa;
use pcoa::nalgebra::DMatrix;

pub fn plagiarism_check(dest_path: &Path) -> Result<()> {
    if dest_path.exists() {
        return Err(Error::new(
            ErrorKind::AlreadyExists,
            "Dest path should not exist",
        ));
    }

    _plagiarism_check(dest_path)?;

    Ok(())
}

fn _plagiarism_check(dest_path: &Path) -> Result<()> {
    // Use Multidimensional scaling to visualize similarities between students
    // https://en.wikipedia.org/wiki/Multidimensional_scaling
    let students = list_students();
    let student_hashes = create_student_hashes(&students)?;
    let distance_matrix = create_distance_matrix(student_hashes);
    let (mut xs, mut ys) = multidimensional_scaling(distance_matrix);
    normalize_vector(&mut xs, 100.0);
    normalize_vector(&mut ys, 100.0);
    create_plagiarism_report(dest_path, &students, &xs, &ys)?;

    Ok(())
}

fn create_student_hashes(students: &[String]) -> Result<Vec<Option<Tlsh>>> {
    let mut student_hashes = Vec::new();
    for student in students {
        let diff_path = student_diff_file(student);
        let student_hash = match process_file(&diff_path)
            .inspect_err(|_| eprintln!("Failed to compute hash for {}", student))
        {
            Ok(r) => Some(r),
            Err(_) => None,
        };
        student_hashes.push(student_hash);
    }
    Ok(student_hashes)
}

fn create_distance_matrix(hashes: Vec<Option<Tlsh>>) -> Vec<Vec<f64>> {
    let mut out: Vec<Vec<f64>> = vec![vec![0.0; hashes.len()]; hashes.len()];
    for i in 0..hashes.len() - 1 {
        for j in i + 1..hashes.len() {
            if let Some(h1) = &hashes[i] {
                if let Some(h2) = &hashes[j] {
                    out[i][j] = h1.diff(h2, true) as f64;
                    out[j][i] = out[i][j];
                }
            }
        }
    }

    out
}

fn multidimensional_scaling(distance_matrix: Vec<Vec<f64>>) -> (Vec<f64>, Vec<f64>) {
    let number_of_dimensions = 2;
    let flat_distance_matrix: Vec<f64> = distance_matrix.iter().flatten().cloned().collect();
    let distance_matrix = DMatrix::from_column_slice(
        distance_matrix.len(),
        distance_matrix.len(),
        &flat_distance_matrix,
    );
    // apply pcoa
    let coords_matrix =
        apply_pcoa(distance_matrix, number_of_dimensions).expect("cannot apply PCoA");

    // NOTE: transpose matrix to get first column for x coordinates and the second - for y coordinates.
    let coords_matrix = coords_matrix.transpose();
    let xs: Vec<f64> = coords_matrix.column(0).iter().copied().collect();
    let ys: Vec<f64> = coords_matrix.column(1).iter().copied().collect();

    (xs, ys)
}

fn normalize_vector(v: &mut [f64], max: f64) {
    // Normalizes vector between [0; max]
    let mn = v
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0.0);
    let mx = v
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0.0);

    if (mx - mn).abs() > f64::EPSILON {
        v.iter_mut().for_each(|a| {
            *a = (*a - mn) / (mx - mn) * max;
        });
    } else {
        v.iter_mut().for_each(|a| {
            *a = 0.0;
        });
    }
}

#[derive(Serialize)]
struct PlagiarismReportContext {
    positions: Vec<StudentContext>,
}

#[derive(Serialize)]
struct StudentContext {
    name: String,
    x: f64,
    y: f64,
}

fn create_plagiarism_report(
    dest_path: &Path,
    students: &[String],
    xs: &[f64],
    ys: &[f64],
) -> Result<()> {
    let mut positions = Vec::new();
    for (i, student) in students.iter().enumerate() {
        positions.push(StudentContext {
            name: student.clone(),
            x: xs[i],
            y: ys[i],
        });
    }
    let s = Handlebars::new()
        .render_template(
            include_str!("../template/plagiarism.hbs"),
            &PlagiarismReportContext { positions },
        )
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    fs::write(dest_path, s)?;

    Ok(())
}

pub fn plagiarism_check_students(student1: &str, student2: &str) -> Result<usize> {
    if !is_student(student1) {
        return Err(Error::new(
            ErrorKind::Other,
            format!("{} is not a student", student1),
        ));
    }

    if !is_student(student2) {
        return Err(Error::new(
            ErrorKind::Other,
            format!("{} is not a student", student2),
        ));
    }

    _plagiarism_check_students(student1, student2)
}

fn _plagiarism_check_students(student1: &str, student2: &str) -> Result<usize> {
    let student1_diff_path = student_diff_file(student1);
    let student2_diff_path = student_diff_file(student2);

    let student1_hash = process_file(&student1_diff_path)?;
    let student2_hash = process_file(&student2_diff_path)?;

    dbg!(&student1_hash.hash(), &student2_hash.hash());

    Ok(student1_hash.diff(&student2_hash, true))
}

fn process_file(path: &Path) -> Result<Tlsh> {
    let file = OpenOptions::new().read(true).open(path)?;
    _process_file(file)
}

fn _process_file(file: File) -> Result<Tlsh> {
    // Luckily, diff processes files in the same order so we don't need to do reordering. However, we should remove all lines for diffing
    let mut builder = TlshBuilder::new(Bucket128, OneByte, Version4);

    let buf = Vec::new();
    let mut reader = BufReader::new(file);
    let mut writer = BufWriter::new(buf);

    buffer_flatmap(&mut reader, &mut writer, |line| {
        let line_copy = line.trim();
        if line_copy.starts_with("diff -ruN")
            || line_copy.starts_with("---")
            || line_copy.starts_with("+++")
            || line_copy.starts_with("@@")
        {
            return None;
        }
        Some(line.to_string())
    })?;

    writer.flush()?;
    builder.update(&writer.into_inner()?);
    builder
        .build()
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))
}
