use std::collections::HashSet;
use rocket::{response::status, serde::json::Json, http::Status};

use crate::{config::darwin_root, list_students::list_students, list_tests::list_tests, types::{TestResultError, TestResults}, view_student_results::parse_test_results};

#[rocket::main]
pub async fn server() -> std::result::Result<(), rocket::Error> {
    let _rocket = rocket::build()
        .mount("/api", routes![get_students, get_tests, get_student_result])
        .launch()
        .await?;

    Ok(())
}

#[get("/students")]
fn get_students() -> Json<Vec<String>> {
    Json(list_students())
}

#[get("/tests")]
fn get_tests() -> Json<HashSet<String>> {
    Json(list_tests())
}

#[get("/result/<student>/<test>")]
fn get_student_result(student: &str, test: &str) -> Result<Json<TestResults>, status::Custom<String>> {
    match parse_test_results(darwin_root().as_path(), student, test) {
        Ok(r) => Ok(Json(r)),
        Err(e) => match e {
            TestResultError::IOError(e) => {
                Err(status::Custom(Status::ImATeapot, e.to_string()))
            }
            TestResultError::TestsNotRun => {
                Err(status::Custom(Status::NotFound, String::from("Test not run")))
            }
        }
    }
}

#[get("/code/<student>")]
fn get_code(student: &str) -> Result<(), status::Custom<String>> {
    Ok(())
}