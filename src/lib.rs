use std::io::Cursor;

use axum::extract::Multipart;
use calamine::{DataType, Reader};
use indexmap::IndexMap;

#[derive(Clone, Debug)]
/// A file is a struct that represents a single file to be merged
pub struct File {
    pub last_modified: String,
    pub name: String,
    pub rows: Vec<Vec<DataType>>,
    pub is_main: bool,
}

impl File {
    /// Creates a new file struct
    pub fn new(
        name: String,
        last_modified: String,
        rows: Vec<Vec<DataType>>,
        is_main: bool,
    ) -> Self {
        File {
            last_modified,
            rows,
            name,
            is_main,
        }
    }
}

/// A files map is a struct that represents an map of files to be merged, and some other options
pub struct FilesMap {
    pub files: IndexMap<String, File>,
    pub sort_by_date: bool,
    pub sort_by_file: bool,
    pub cutting_rows: usize,
}

impl FilesMap {
    /// Creates a new files map struct from a multipart form argument
    pub async fn new(mut multipart: Multipart) -> Self {
        let mut files_to_merge: IndexMap<String, File> = IndexMap::new();
        let mut cutting_rows: usize = 0;
        let mut sort_by_date: bool = false;
        let mut sort_by_file: bool = false;

        while let Some(field) = multipart.next_field().await.unwrap() {
            let content_type = field.content_type().map(str::to_owned);

            let name = field.name().unwrap_or("unknown").to_owned();
            let other_name = field.file_name().unwrap_or("unknown").to_owned();
            let bytes = field.bytes().await.unwrap();

            if name.starts_with("sort_by") {
                if name == "sort_by_date" {
                    let val = String::from_utf8(bytes.to_vec())
                        .unwrap()
                        .parse::<bool>()
                        .unwrap();

                    if val {
                        sort_by_date = true;
                    }
                } else if name == "sort_by_file" {
                    let val = String::from_utf8(bytes.to_vec())
                        .unwrap()
                        .parse::<bool>()
                        .unwrap();

                    if val {
                        sort_by_file = true;
                    }
                }
                continue;
            }

            if name.ends_with("(LM)") {
                let date = String::from_utf8(bytes.to_vec()).unwrap();
                // println!("Last modified (str): {}", &date);

                let mut file_name = name.split("(LM)").next().unwrap().to_string();
                let mut is_main = false;

                if name.contains("MAIN") {
                    println!("That's the main file.");
                    file_name = file_name.replace("-MAIN", "");
                    is_main = true;
                }

                // println!("File name (lm): {:?}", &file_name);

                if files_to_merge.contains_key(&file_name) {
                    let val = files_to_merge.get_mut(&file_name).unwrap();
                    val.last_modified = date;
                } else {
                    files_to_merge.insert(
                        file_name.to_owned(),
                        File::new(other_name.clone(), date, Vec::new(), is_main),
                    );
                }

                continue;
            }

            if name == "cuttingRows" {
                let cutting_rows_str = String::from_utf8(bytes.to_vec()).unwrap();
                println!("Cutting rows (str): {}", &cutting_rows_str);

                if !cutting_rows_str.is_empty() {
                    cutting_rows = cutting_rows_str.parse::<usize>().unwrap();
                    println!("Cutting rows (usize): {}", &cutting_rows);
                }

                continue;
            }

            if content_type
                == Some(
                    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
                )
                || content_type == Some("application/vnd.ms-excel".to_string())
            {
                let bytes = bytes.to_vec();
                let mut is_main = false;

                let reader = Cursor::new(bytes);
                let mut workbook = calamine::open_workbook_auto_from_rs(reader).unwrap();

                println!("File name (excel): {:?}", &name);

                if name.contains("MAIN") {
                    println!("That's the main file. (excel)");
                    is_main = true;
                }

                if let Some(range) = workbook.worksheet_range_at(0) {
                    let sheet = range.unwrap();
                    let rows: Vec<_> = sheet
                        .to_owned()
                        .rows()
                        .map(|slice| slice.to_vec())
                        .collect();

                    // print cutting rows
                    println!("Cutting Rows: {:?}", cutting_rows);

                    if files_to_merge.contains_key(&other_name) {
                        let val = files_to_merge.get_mut(&other_name).unwrap();
                        val.rows = rows;
                        val.is_main = is_main;
                    } else {
                        files_to_merge.insert(
                            name.replace("-MAIN", "").to_owned(),
                            File::new(other_name.clone(), "unkown".to_string(), rows, is_main),
                        );
                    }
                }

                continue;
            }
        }

        Self {
            files: files_to_merge,
            sort_by_date,
            sort_by_file,
            cutting_rows,
        }
    }
}
