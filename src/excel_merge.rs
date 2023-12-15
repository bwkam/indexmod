use std::{
    fs::{self, File},
    io::Cursor,
    path::Path,
    sync::Arc,
};

use crate::error::Result;
use axum::{extract::Multipart, response::IntoResponse};
// use axum_macros::debug_handler;
use calamine::{open_workbook_auto_from_rs, DataType};
use chrono::NaiveDateTime;
use excel_merge::create_file_vec;
use rust_xlsxwriter::Workbook;
use std::io::Write;
use std::sync::Mutex;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(merge_files))]
pub struct ApiDoc;

#[utoipa::path(
    get,
    path = "/merge",
    responses(
        (status = 200, description = "Merge Excel files")
    )
)]
pub async fn merge_files(multipart: Multipart) -> Result<impl IntoResponse> {
    println!("Merge requested. Processing files...");

    let tuple_result = create_file_vec(multipart).await;

    let mut files_to_merge = tuple_result.0;
    let sort_by_date = tuple_result.1;
    let sort_by_file = tuple_result.2;
    let cutting_rows = tuple_result.3;

    // before
    println!("Before sorting...");
    files_to_merge
        .iter()
        .for_each(|(_, v)| println!("{:?}", &v.name));

    if sort_by_date {
        print!("Sorting by date...");
        files_to_merge.sort_by(|_, v1, _, v2| {
            let dt1 = NaiveDateTime::parse_from_str(&v1.last_modified, "%Y %m %d %H%M").unwrap();
            let dt2 = NaiveDateTime::parse_from_str(&v2.last_modified, "%Y %m %d %H%M").unwrap();
            dt1.cmp(&dt2)
        });
    } else if sort_by_file {
        println!("Sorting by file...");
        files_to_merge.sort_by(|_, b, _, d| {
            let a_name = b
                .name
                .clone()
                .to_ascii_lowercase()
                .replace(".xlsx", "")
                .replace(".xls", "");
            let b_name = d
                .name
                .clone()
                .to_ascii_lowercase()
                .replace(".xlsx", "")
                .replace(".xls", "");
            let a_chars = a_name.chars().collect::<Vec<char>>();
            let b_chars = b_name.chars().collect::<Vec<char>>();
            let mut i = 0;
            while i < a_chars.len() && i < b_chars.len() {
                let a_char = a_chars[i];
                let b_char = b_chars[i];
                if a_char.is_ascii_digit() && b_char.is_ascii_digit() {
                    let a_num = a_char.to_digit(10).unwrap();
                    let b_num = b_char.to_digit(10).unwrap();
                    if a_num != b_num {
                        return a_num.cmp(&b_num);
                    }
                } else if a_char != b_char {
                    return a_char.cmp(&b_char);
                }
                i += 1;
            }
            a_chars.len().cmp(&b_chars.len())
        });
    }

    // after
    println!("After sorting...");
    files_to_merge.sort_by_cached_key(|_, v| !v.is_main);
    files_to_merge
        .iter()
        .for_each(|(_, v)| println!("{:?}", &v.name));

    let names = files_to_merge
        .values()
        .map(|x| x.name.clone())
        .collect::<Vec<String>>();

    files_to_merge
        .iter()
        .filter(|file| file.1.is_main)
        .for_each(|file| {
            println!("Main file: {:?}", file.0);
        });

    let mut stats = File::create("rows.txt").unwrap();
    files_to_merge.iter().for_each(|(_, v)| {
        writeln!(&mut stats, "Name: {:?}, rows: {:?}", v.name, v.rows.len()).unwrap();
    });

    let mut first_rows: Vec<String> = files_to_merge
        .iter()
        .next()
        .unwrap()
        .1
        .rows
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

    println!("Merging files...");

    println!(
        "Date: {:?}",
        files_to_merge.iter().next().unwrap().1.last_modified
    );

    // Create a directory if it doesn't exist
    let directory = Path::new("text");
    if !directory.exists() {
        std::fs::create_dir(directory).unwrap();
    } else {
        // Delete all files in the directory
        fs::remove_dir_all(directory).unwrap();
        std::fs::create_dir(directory).unwrap();
    }

    // cut n rows from each non-main file
    if cutting_rows > 0 {
        files_to_merge
            .iter_mut()
            .filter(|x| !x.1.is_main)
            .for_each(|(_, v)| {
                v.rows = v.rows.drain((cutting_rows - 1)..).collect();
            });
    }

    let mut acc_width = 0;
    let mut values_rows: Vec<Vec<String>> = files_to_merge
        .iter()
        .enumerate()
        .flat_map(|(i, (_, inner_vec))| {
            let main_data = inner_vec
                .rows
                .iter()
                .skip(1)
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

                    intro_headers.push(inner_vec.last_modified.to_owned());
                    intro_headers.push((i + 1).to_string());
                    intro_headers.push((acc_width + 1).to_string());
                    intro_headers.push((i + 1).to_string() + "-" + ((j) + 1).to_string().as_str());
                    intro_headers.push(inner_vec.name.replace("-MAIN", ""));

                    let mut vec: Vec<String> = vec![];
                    vec.append(&mut intro_headers);
                    vec.append(&mut cur_row_values);

                    acc_width += 1;

                    vec
                })
                .collect::<Vec<Vec<String>>>();

            let directory = Path::new("text");
            let file_path = directory.join(format!("{}.txt", &inner_vec.name));

            // Open the file in write mode
            let mut file = File::create(&file_path).unwrap();

            for row in &main_data {
                let line = row.join(",");
                writeln!(&mut file, "{}", line).unwrap();
            }

            main_data
        })
        .collect();

    // modify the headers
    let mut extra_headers = [
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

    let workbook: Arc<Mutex<Workbook>> = Arc::new(Mutex::new(Workbook::new()));
    let workbook_clone = workbook.clone();

    println!("Writing to file...");
    tokio::task::spawn_blocking(move || {
        let mut workbook_clone_locked = workbook_clone.lock().unwrap();
        let worksheet = workbook_clone_locked.add_worksheet();
        worksheet.write_row_matrix(0, 0, &values_rows).unwrap();
    })
    .await
    .unwrap();

    println!("Saving and sending file to the client for download...");

    let mut missing_files = vec![];

    // check if all files are there
    for name in &names {
        let file_path = format!("text/{}.txt", name);
        if fs::metadata(&file_path).is_err() {
            missing_files.push(name.to_string());
        }
    }

    if !missing_files.is_empty() {
        println!(
            "[FAILURE] The following files are missing: {:?}",
            missing_files
        );
    } else {
        println!("[PASS] All files are there.")
    }

    let workbook_clone = workbook.clone();
    let mut workbook_clone_locked = workbook_clone.lock().unwrap();

    workbook_clone_locked.save("output.xlsx").unwrap();

    Ok(workbook_clone_locked.save_to_buffer().unwrap().to_vec())
}
