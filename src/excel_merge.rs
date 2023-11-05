use std::{collections::HashMap, io::Cursor, sync::Arc};

use anyhow::Result;
use axum::{
    extract::Multipart,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use calamine::{open_workbook_from_rs, DataType, Reader, Xlsx};
use chrono::{DateTime, NaiveDateTime, Utc};
use rust_xlsxwriter::Workbook;
use utoipa::OpenApi;

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

#[derive(Clone, Debug)]
pub struct FileData {
    last_modified: String,
    data: Vec<Vec<DataType>>,
}

impl FileData {
    pub fn new(lm: String, data: Vec<Vec<DataType>>) -> Self {
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

    while let Some(field) = multipart.next_field().await.unwrap() {
        let content_type = field.content_type().map(str::to_owned);

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
            let cutting_rows_str = String::from_utf8(bytes.to_vec()).unwrap();

            if !cutting_rows_str.is_empty() {
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
        .next()
        .unwrap()
        .1
        .data
        .first()
        .unwrap()
        .iter()
        .map(|data| match data {
            DataType::String(s) => s.to_string(),
            DataType::Float(f) => f.to_string(),
            DataType::Int(i) => i.to_string(),
            DataType::Bool(b) => b.to_string(),
            DataType::Error(e) => e.to_string(),
            DataType::Empty => "".to_string(),
            _ => "".to_string(),
        })
        .collect();

    let mut values_rows: Vec<Vec<String>> = rows_hash
        .into_iter()
        .enumerate()
        .map(|(i, (name, inner_vec))| {
            let main_data = inner_vec
                .data
                .iter()
                .skip((1 + cutting_rows) as usize)
                .enumerate()
                .map(|(j, file)| {
                    let mut intro_headers = vec![];
                    let mut cur_row_values: Vec<String> = file
                        .iter()
                        .map(|row_data| match row_data {
                            DataType::String(s) => s.to_string(),
                            DataType::Float(f) => f.to_string(),
                            DataType::Int(i) => i.to_string(),
                            DataType::Bool(b) => b.to_string(),
                            DataType::Error(e) => e.to_string(),
                            DataType::Empty => "".to_string(),
                            _ => "".to_string(),
                        })
                        .collect();

                    intro_headers.push((&inner_vec.last_modified).to_owned());
                    intro_headers.push(i.to_string());
                    intro_headers.push(j.to_string());
                    intro_headers.push(j.to_string() + "-" + i.to_string().as_str());
                    intro_headers.push(name.to_string());

                    let mut vec: Vec<String> = vec![];
                    vec.append(&mut intro_headers);
                    vec.append(&mut cur_row_values);
                    vec
                })
                .collect::<Vec<Vec<String>>>();

            main_data
        })
        .flatten()
        .collect();

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

    values_rows.insert(0, extra_headers);

    // TODO: put this in a seperate thread
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.write_row_matrix(0, 0, &values_rows)?;
    workbook.save("output.xlsx").unwrap();

    Ok(workbook.save_to_buffer().unwrap().to_vec())
}
