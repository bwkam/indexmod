use anyhow::Context;
use std::io::Cursor;

use crate::{error::Result, reply::ReplyFiles};
use crate::{process_workbook, FilesMap};
use axum::{
    extract::{Multipart, Query},
    response::IntoResponse,
};
use calamine::Dimensions;
use itertools::Itertools;
use rust_xlsxwriter::Workbook;
use serde::Deserialize;
use size::Size;
use tracing::{info, trace};

#[derive(Deserialize, Debug)]
pub struct Params {
    pub reply: bool,
}

// TODO: Add an #[instrument] for span tracing
#[utoipa::path(
    get,
    path = "/reply",
    responses(
        (status = 200, description = "Cell reply")
    )
)]
pub async fn cell_reply_files(
    Query(params): Query<Params>,
    multipart: Multipart,
) -> Result<impl IntoResponse> {
    info!("Cell reply requested. Processing files...");

    // create the files map object that will handle the "cell reply"
    let buffer = FilesMap::reply_from_multipart(multipart, params.reply)
        .await?
        .write_to_buffer(false, params.reply)?;

    Ok(buffer)
}

#[utoipa::path(
    get,
    path = "/reply-single",
    responses(
        (status = 200, description = "Cell reply")
    )
)]
pub async fn cell_reply_file(
    Query(params): Query<Params>,
    multipart: Multipart,
) -> Result<impl IntoResponse> {
    info!("Cell reply requested (single). Processing file...");

    // create the files map object that will handle the "cell reply"
    let buffer = FilesMap::reply_from_multipart(multipart, params.reply)
        .await?
        .write_to_buffer(true, params.reply)?;

    Ok(buffer)
}

#[utoipa::path(
    get,
    path = "/reply-template",
    responses(
        (status = 200, description = "Cell reply template")
    )
)]
pub async fn cell_reply_template(mut multipart: Multipart) -> Result<impl IntoResponse> {
    info!("Cell reply template requested. Processing file...");

    let mut files: ReplyFiles = ReplyFiles::new(vec![]);
    let mut dates: Vec<String> = vec![];
    let mut cutting_rows: Vec<u32> = vec![];
    let mut sizes: Vec<u32> = vec![];

    while let Some(field) = multipart.next_field().await.unwrap() {
        let content_type = field.content_type().map(str::to_owned);

        let name = field.name().unwrap_or("unknown").to_owned();
        let other_name = field.file_name().unwrap_or("unknown").to_owned();
        let bytes = field.bytes().await.unwrap();

        if name == "last-mod[]" {
            let date = String::from_utf8(bytes.to_vec()).unwrap();

            dates.push(date);

            continue;
        }

        if name == "size[]" {
            if let Ok(size_str) = String::from_utf8(bytes.to_vec()) {
                if let Ok(size) = size_str.parse::<u32>() {
                    sizes.push(size);
                }
            }
            trace!("Size values: {:?}", sizes);
        }

        if name == "cut-row[]" {
            if let Ok(cut_row_str) = String::from_utf8(bytes.to_vec()) {
                if let Ok(cut_row) = cut_row_str.parse::<u32>() {
                    cutting_rows.push(cut_row);
                }
            }
            trace!("Cutting-rows: {:?}", cutting_rows);
        }

        if let Some(content_type) = content_type {
            let bytes = bytes.to_vec();
            let reader = Cursor::new(bytes);

            match content_type.as_str() {
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
                    let mut workbook: calamine::Xlsx<_> =
                        calamine::open_workbook_from_rs(reader).unwrap();
                    println!("File name (xlsx): {:?}", &name);
                    let mut merged_regions: Vec<Dimensions> = vec![];
                    if workbook.load_merged_regions().is_ok() {
                        // FIXME: don't use to_owned
                        merged_regions = workbook
                            .merged_regions()
                            .to_owned()
                            .iter()
                            .map(|region| region.2)
                            .collect();
                    }
                    process_workbook(&mut workbook, &other_name, &mut files, &merged_regions);
                }
                "application/vnd.ms-excel" => {
                    let mut workbook: calamine::Xls<_> =
                        calamine::open_workbook_from_rs(reader).unwrap();
                    println!("File name (xls): {:?}", &name);
                    process_workbook(&mut workbook, &other_name, &mut files, &vec![]);
                }
                _ => {
                    // Handle other content types or errors
                }
            }
        }
    }

    files.data.iter_mut().enumerate().for_each(|(i, file)| {
        file.last_modified = dates[i].clone();
        file.cutting_rows = cutting_rows[i].clone();
        file.size = sizes[i].clone();
    });

    dates.clear();
    cutting_rows.clear();
    sizes.clear();

    // dbg!(&files.data);

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let intro_headers = vec![
        "Series No",
        "File Name",
        "File Extension",
        "Last Modified Date",
        "Size",
        "Cut row",
    ]
    .iter_mut()
    .map(|x| x.to_string())
    .collect_vec();

    let rows = files
        .data
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let size = Size::from_bytes(file.size).to_string();
            vec![
                i.to_string(),
                file.name.to_string(),
                file.ext.to_string(),
                file.last_modified.to_string(),
                size,
                file.cutting_rows.to_string(),
            ]
        })
        .collect_vec();

    let all_rows = std::iter::once(intro_headers).chain(rows).collect_vec();

    worksheet
        .write_row_matrix(0, 0, all_rows)
        .context("error writing template rows")?;

    let buffer = workbook
        .save_to_buffer()
        .context("failed to save workbook to buffer")?;

    Ok(buffer)
}
