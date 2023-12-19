use std::io::Write;
use std::{io::Cursor, path::Path};

use crate::error::{Error, Result};
use anyhow::Context;
use axum::extract::Multipart;
use calamine::{DataType, Range, Reader};
use chrono::NaiveDateTime;
use indexmap::IndexMap;
use rust_xlsxwriter::Workbook;

pub mod error;
pub mod excel_merge;

#[derive(Clone, Debug)]
enum SortBy {
    Date,
    File,
}

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
    pub sheet_range: Option<Range<DataType>>,
}

impl FilesMap {
    /// Creates a new files map struct from a multipart form argument
    pub async fn new(mut multipart: Multipart) -> Result<Self> {
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

                if workbook.worksheets().len() > 1 {
                    return Err(Error::SheetLimitExceeded);
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

        Ok(Self {
            files: files_to_merge,
            sort_by_date,
            sort_by_file,
            cutting_rows,
            sheet_range: None,
        })
    }

    /// sort based on date or file name
    fn sort(&mut self, sort: SortBy) {
        match sort {
            SortBy::Date => {
                print!("Sorting by date...");
                self.files.sort_by(|_, v1, _, v2| {
                    let dt1 =
                        NaiveDateTime::parse_from_str(&v1.last_modified, "%Y %m %d %H%M").unwrap();
                    let dt2 =
                        NaiveDateTime::parse_from_str(&v2.last_modified, "%Y %m %d %H%M").unwrap();
                    dt1.cmp(&dt2)
                });
            }

            SortBy::File => {
                println!("Sorting by file...");
                self.files.sort_by(|_, b, _, d| {
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
        }
    }

    /// merge files into a `Vec<Vec<String>>`
    fn merge(&mut self) -> Result<Vec<Vec<String>>> {
        let mut files_to_merge = self.files.clone();
        let sort_by_date = self.sort_by_date;
        let sort_by_file = self.sort_by_file;
        let cutting_rows = self.cutting_rows;

        files_to_merge
            .iter()
            .for_each(|(_, v)| println!("{:?}", &v.name));

        if sort_by_date {
            self.sort(SortBy::Date);
        } else if sort_by_file {
            self.sort(SortBy::File);
        }

        // sorting that will run anyways
        files_to_merge.sort_by_cached_key(|_, v| !v.is_main);
        files_to_merge
            .iter()
            .for_each(|(_, v)| println!("{:?}", &v.name));

        files_to_merge
            .iter()
            .filter(|file| file.1.is_main)
            .for_each(|file| {
                println!("Main file: {:?}", file.0);
            });

        let mut stats = std::fs::File::create("rows.txt").unwrap();
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
            std::fs::remove_dir_all(directory).unwrap();
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
                        intro_headers
                            .push((i + 1).to_string() + "-" + ((j) + 1).to_string().as_str());
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
                let mut file = std::fs::File::create(&file_path).unwrap();

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

        Ok(values_rows)
    }

    /// save the merged files to a buffer
    pub fn save_to_buf(&mut self) -> Result<Vec<u8>> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        let values_rows = self.merge()?;

        worksheet
            .write_row_matrix(0, 0, &values_rows)
            .context("Failed to write to workbook")?;

        let buf = workbook
            .save_to_buffer()
            .context("Failed to save workbook to buffer")?
            .to_vec();

        Ok(buf)
    }
}
