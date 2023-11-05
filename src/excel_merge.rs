#![allow(unused)]

use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Cursor},
    ops::Add,
    sync::Arc,
};

use anyhow::{Error, Result};
use axum::{
    body::Bytes,
    extract::{Multipart, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use calamine::{open_workbook, open_workbook_from_rs, DataType, Range, Reader, Rows, Xlsx};
use rust_xlsxwriter::Workbook;
use utoipa::OpenApi;

use crate::AppState;
// #[derive(Deserialize, Debug)]
// pub struct Data {
//     data: Vec<Vec<serde_json::Map<String, serde_json::Value>>>, // fix this
// }

#[derive(Deserialize, Debug)]
pub struct Data {
    data: Vec<Vec<serde_json::Map<String, serde_json::Value>>>,
}

#[derive(Serialize, Debug)]
struct Message {
    message: String,
}

type MyRows<'a> = Vec<calamine::Rows<'a, DataType>>;

// Make our own error that wraps `anyhow::Error`.
pub struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[derive(OpenApi)]
#[openapi(paths(merge_files))]
pub struct ApiDoc;

#[utoipa::path(
    post,
    path = "/add",
    responses(
        (status = 200, description = "Add excel data to merge")
    )
)]
pub async fn add_file(
    State(state): State<AppState>,
    Json(data): Json<Data>,
) -> Result<impl IntoResponse, AppError> {
    let mut d = state.data.lock().unwrap();
    data.data.iter().for_each(|x| d.push(x.clone()));

    println!("->> Data added to merge");

    let msg = Message {
        message: "Data added to merge".to_string(),
    };

    Ok(Json(msg))
}

#[derive(Clone, Debug)]
pub struct FileData {
    last_modified: u32,
    data: Vec<Vec<DataType>>,
}

impl FileData {
    pub fn new(lm: u32, data: Vec<Vec<DataType>>) -> Self {
        FileData {
            last_modified: lm,
            data,
        }
    }
}

#[utoipa::path(
    get,
    path = "/merge",
    responses(
        (status = 200, description = "Merge Excel files")
    )
)]
pub async fn merge_files(mut multipart: Multipart) -> Result<impl IntoResponse, AppError> {
    let mut rows_hash: HashMap<String, FileData> = HashMap::new();
    let mut cutting_rows: u32 = 0;

    let rows_hash_clone = rows_hash.clone();

    while let Some(mut field) = multipart.next_field().await.unwrap() {
        println!("In the loop here.");
        let content_type = field.content_type().map(str::to_owned);
        field.headers().iter().for_each(|x| {
            println!("Header: {:?}", x);
        });
        let name = field.name().unwrap_or("unknown").to_owned();
        let other_name = field.file_name().unwrap_or("unknown").to_owned();
        let bytes = field.bytes().await.unwrap();

        if name.ends_with("LM") {
            let last_modified_str = String::from_utf8(bytes.to_vec()).unwrap();
            let last_modified_i64 = last_modified_str.parse::<i64>().unwrap();

            let naive = NaiveDateTime::from_timestamp_millis(last_modified_i64).unwrap();
            let datetime: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive, Utc);

            let date = format!("{}", datetime.format("%Y %m %d %H%M"));

            let file_name = name.split("-").next().unwrap().to_string();

            if rows_hash.contains_key(&file_name) {
                let val = rows_hash.get_mut(&file_name).unwrap();
                val.last_modified = date;
            } else {
                rows_hash.insert(file_name.to_owned(), FileData::new(date, Vec::new()));
            }
        }

        if name == "cuttingRows" {
            println!("Setting cutting rows.");

            let cutting_rows_str = String::from_utf8(bytes.to_vec()).unwrap();

            if !cutting_rows_str.is_empty() {
                println!("Cutting rows: {:?}", cutting_rows);
                cutting_rows = cutting_rows_str.parse::<u32>().unwrap();
            }
        }

        if content_type
            == Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string())
        {
            let task = tokio::task::spawn_blocking(move || {
                let bytes = bytes.to_vec();
                let reader = Cursor::new(bytes);
                let mut workbook: Xlsx<_> = open_workbook_from_rs(reader).unwrap();
                if let Some(range) = workbook.worksheet_range_at(0) {
                    let sheet = range.unwrap();
                    let rows: Vec<_> = sheet
                        .to_owned()
                        .rows()
                        .into_iter()
                        .map(|slice| slice.to_vec())
                        .collect();

                    if rows_hash.contains_key(&other_name) {
                        let val = rows_hash.get_mut(&other_name).unwrap();
                        val.data = rows;
                    } else {
                        rows_hash.insert(
                            other_name.to_owned(),
                            FileData::new("unkown".to_string(), rows),
                        );
                    }
                }
            });

            task.await.unwrap();
        }
    }

    let rows_hash_clone = rows_hash.clone();
    let rows_hash_clone_locked = rows_hash_clone.lock().await;

    println!("Cur rows: {:?}", rows_hash_clone_locked);

    let mut first_rows: Vec<String> = rows_hash_clone_locked
        .iter()
        .map(|(_, inner_vec)| {
            let first_row = inner_vec.first().unwrap();
            first_row
                .iter()
                .map(|rowData| match rowData {
                    DataType::String(s) => s.to_string(),
                    DataType::Float(f) => f.to_string(),
                    DataType::Int(i) => i.to_string(),
                    DataType::Bool(b) => b.to_string(),
                    DataType::Error(e) => e.to_string(),
                    DataType::Empty => "".to_string(),
                    _ => "".to_string(),
                })
                .collect::<Vec<String>>()
        })
        .flatten()
        .collect();

    let mut values_rows: Vec<Vec<String>> = rows_hash
        .into_iter()
        .enumerate()
        .map(|(i, (name, inner_vec))| {
            let mut main_data = inner_vec
                .iter()
                .skip((1 + cutting_rows) as usize) // skip the first row of the inner vector
                .enumerate()
                .map(|(j, file)| {
                    let mut intro_headers = vec![];
                    let mut cur_row_values: Vec<String> = file
                        .iter()
                        .map(|rowData| match rowData {
                            DataType::String(s) => s.to_string(),
                            DataType::Float(f) => f.to_string(),
                            DataType::Int(i) => i.to_string(),
                            DataType::Bool(b) => b.to_string(),
                            DataType::Error(e) => e.to_string(),
                            DataType::Empty => "".to_string(),
                            _ => "".to_string(),
                        })
                        .collect();

                    intro_headers.push("2023".to_string());
                    intro_headers.push(i.to_string());
                    intro_headers.push(j.to_string());
                    intro_headers.push(j.to_string() + "-" + i.to_string().as_str());
                    intro_headers.push(name.to_string());

                    let mut vec: Vec<String> = vec![];
                    println!("Intro headers: {:?}", intro_headers);
                    vec.append(&mut intro_headers);
                    // print intro headers

                    // print the main data
                    println!("Main row data: {:?}", cur_row_values);
                    vec.append(&mut cur_row_values);

                    vec

                    // cur_row_values
                    // intro_headers.append(&mut cur_row_values);
                    // intro_headers
                })
                .collect::<Vec<Vec<String>>>();

            main_data
        })
        .flatten()
        .collect();

    // print values in rows
    // values_rows.iter().for_each(|x| println!("{:?}", x));

    // modify the headers
    let mut extra_headers = vec![
        "Date Modified",
        "Number of Files",
        "Series Number",
        "Count Number",
        "File Name",
    ]
    .iter()
    .map(|x| x.to_string())
    .collect::<Vec<String>>();

    extra_headers.append(&mut first_rows);

    // println!("Extra headers: {:?}", extra_headers);

    values_rows.insert(0, extra_headers);

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.write_row_matrix(0, 0, &values_rows);
    workbook.save("output.xlsx").unwrap();

    Ok("Success".to_string())
}
