use std::collections::HashSet;
use rocket::serde::json::Json;

use crate::{config::darwin_path, list_students::list_students, list_tests::list_tests};

#[rocket::main]
pub async fn server() -> std::result::Result<(), rocket::Error> {
    let _rocket = rocket::build()
        .mount("/api", routes![get_students, get_tests])
        .launch()
        .await?;

    Ok(())
}

#[get("/students")]
fn get_students() -> Json<Vec<String>> {
    Json(list_students(darwin_path().as_path()))
}

#[get("/tests")]
fn get_tests() -> Json<HashSet<String>> {
    Json(list_tests(darwin_path().as_path()))
}