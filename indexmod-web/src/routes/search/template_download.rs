use anyhow::Context;
use askama_axum::IntoResponse;
use axum::extract::Multipart;
use rust_xlsxwriter::Workbook;
use tracing::info;

use crate::{error::Result, Conditions};

#[utoipa::path(
    get,
    path = "/search",
    responses(
        (status = 200, description = "Query excel files")
    )
)]
pub async fn download(mut multipart: Multipart) -> Result<impl IntoResponse> {
    info!("Search template download requested. Processing...");

    let mut conditions: Conditions = Conditions { conditions: vec![] };
    let mut vec_to_write: Vec<Vec<String>> = vec![];

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or("unknown").to_owned();
        let bytes = field.bytes().await.unwrap();

        if name == "template" {
            conditions =
                serde_json::from_slice(bytes.as_ref()).context("failed to serialize conditions")?;
        }
    }

    for condition in &conditions.conditions {
        let mut vec = vec![];

        // data
        vec.push(condition.data.clone());

        // title if found
        if let Some(title) = &condition.title {
            if !title.is_empty() {
                vec.push(title.to_owned());
            }
        }

        // intersections if not empty
        if !condition.intersections.is_empty() {
            for condition in &condition.intersections {
                println!("pushing data to template");
                vec.push(condition.data.clone());
                if let Some(title) = &condition.title {
                    println!("title: {}", title);

                    if !title.is_empty() {
                        println!("not empty, so we are pushing it");
                        vec.push(title.clone());
                    }
                }
            }
        }

        vec_to_write.push(vec);
    }

    dbg!(&vec_to_write);

    let mut headers: Vec<String> = vec![];

    // get the longest row in the vec_to_write vec
    let longest_row = vec_to_write.iter().map(|row| row.len()).max().unwrap();

    // create the headers
    for _ in 0..(longest_row + 7) {
        headers.push("DATA".to_string());
        headers.push("TITLE".to_string());
    }

    // write the headers to the vec_to_write vec
    vec_to_write.insert(0, headers);

    // create a new workbook and worksheet and write the vec_to_write vec to it
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet
        .write_row_matrix(0, 0, &vec_to_write)
        .context("failed to write row matrix")?;

    let buffer = workbook
        .save_to_buffer()
        .context("failed to save workbook to buffer")?;

    Ok(buffer)
}
