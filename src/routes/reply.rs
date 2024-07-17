use crate::error::Error;
use anyhow::Context;
use std::io::Cursor;

use crate::{error::Result, reply::ReplyFiles};
use crate::{get_file_extension, process_workbook, FilesMap};
use axum::{
    extract::{Multipart, Query},
    response::IntoResponse,
};
use calamine::{Data, Dimensions, Reader};
use itertools::Itertools;
use rust_xlsxwriter::Workbook;
use serde::Deserialize;
use size::Size;
use tracing::{debug, info, trace, warn};

// TODO: Add an #[instrument] for span tracing
#[utoipa::path(
    get,
    path = "/reply",
    responses(
        (status = 200, description = "Cell reply")
    )
)]
pub async fn cell_reply_files(multipart: Multipart) -> Result<impl IntoResponse> {
    info!("Cell reply requested. Processing files...");

    // create the files map object that will handle the "cell reply"
    let buffer = FilesMap::reply_from_multipart(multipart)
        .await?
        .write_to_buffer(false)?;

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
    multipart: Multipart,
) -> Result<impl IntoResponse> {
    info!("Cell reply requested (single). Processing file...");

    // create the files map object that will handle the "cell reply"
    let buffer = FilesMap::reply_from_multipart(multipart)
        .await?
        .write_to_buffer(true)?;

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
    let mut checked: Vec<bool> = vec![];

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

        if name == "checked[]" {
            if let Ok(checked_str) = String::from_utf8(bytes.to_vec()) {
                match checked_str.as_str() {
                    "Y" => checked.push(true),
                    "N" | "" => checked.push(false),
                    _ => checked.push(false),
                }
            }
            trace!("checked: {:?}", checked);

            continue;
        }

        if let Some(content_type) = content_type {
            let bytes = bytes.to_vec();
            let reader = Cursor::new(bytes);

            if content_type.as_str()
                == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                || content_type.as_str() == "application/vnd.ms-excel"
            {
                println!("File name: {:?}", &name);

                let mut workbook = calamine::open_workbook_auto_from_rs(reader).unwrap();

                if workbook.worksheets().len() > 1 {
                    warn!("Has more than one sheet! Will only parse the first sheet.");
                    return Err(Error::SheetLimitExceeded);
                } else {
                    debug!("Has only one sheet.")
                }

                let sheet_name = &workbook.worksheets()[0];

                let other_name_clone = other_name.to_owned();
                let ext = get_file_extension(&other_name_clone).unwrap();

                files.data.push(crate::reply::ReplyFile::new(
                    other_name.to_owned(),
                    "unknown".to_string(),
                    vec![],
                    ext.to_string(),
                    0,
                    0,
                    vec![],
                    vec![],
                    vec![],
                    false,
                    sheet_name.0.to_string(),
                    false,
                    false,
                ));
            }
        }
    }

    files.data.iter_mut().enumerate().for_each(|(i, file)| {
        file.last_modified = dates[i].clone();
        file.cutting_rows = cutting_rows[i].clone();
        file.size = sizes[i].clone();
        file.checked = checked[i].clone();
    });

    dates.clear();
    cutting_rows.clear();
    sizes.clear();
    checked.clear();

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
        "Cell Reply",
    ]
    .iter_mut()
    .map(|x| Data::String(x.to_string()))
    .collect_vec();

    let rows = files
        .data
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let checked_value = match file.checked {
                true => "Y".to_string(),
                false => "".to_string(),
            };

            let size = Size::from_bytes(file.size).to_string();
            vec![
                Data::String(i.to_string()),
                Data::String(
                    file.name
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                ),
                Data::String(file.ext.to_string()),
                Data::String(file.last_modified.to_string()),
                Data::String(size),
                Data::String(file.cutting_rows.to_string()),
                Data::String(checked_value),
            ]
        })
        .collect_vec();

    let all_rows = std::iter::once(intro_headers).chain(rows).collect_vec();

    for (i, row) in all_rows.iter().enumerate() {
        for (j, cell) in row.iter().enumerate() {
            worksheet
                .write_string(i as u32, j as u16, cell.to_string())
                .context("error writing template cell")?;
        }
    }

    let buffer = workbook
        .save_to_buffer()
        .context("failed to save workbook to buffer")?;

    Ok(buffer)
}
