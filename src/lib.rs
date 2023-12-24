use std::{borrow::Cow, io::Cursor, path::Path};

use crate::error::{Error, Result};
use crate::merge::MergeFiles;

use axum::extract::Multipart;
use calamine::{DataType, Reader};
use chrono::NaiveDateTime;
use indexmap::IndexMap;
use search::{Search, SearchFiles};
use std::io::Write;

pub mod api;
pub mod error;
pub mod merge;
pub mod routes;
pub mod search;

#[derive(Clone, Debug)]
enum SortBy {
    Date,
    File,
}

// TODO: Implement IntoExcelData and become sane
#[derive(Clone, Debug)]
/// A file is a struct that represents a single file to be merged
pub struct File {
    pub last_modified: String,
    pub name: String,
    // pub rows: Vec<Vec<String>>,
    // BREAKING:
    pub rows: Vec<Cow<'static, [DataType]>>,
    pub is_main: bool,
}

impl File {
    /// Creates a new file struct
    pub fn new(
        name: String,
        last_modified: String,
        rows: Vec<Cow<'static, [DataType]>>,
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
    pub fn new(
        files: IndexMap<String, File>,
        sort_by_date: bool,
        sort_by_file: bool,
        cutting_rows: usize,
    ) -> Result<Self> {
        Ok(Self {
            files,
            sort_by_date,
            sort_by_file,
            cutting_rows,
        })
    }

    // TODO: impl From<Multipart> for FilesMap, i.e make this a more sort of library-generic function
    /// Creates a new files map struct from a multipart form argument
    pub async fn from_multipart(mut multipart: Multipart) -> Result<FilesMap> {
        let mut files_to_merge = IndexMap::new();
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
                    let val: &mut File = files_to_merge.get_mut(&file_name).unwrap();
                    val.last_modified = date;
                } else {
                    files_to_merge.insert(
                        file_name.to_owned(),
                        File::new(other_name.clone(), date, vec![], is_main),
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
                    let rows = sheet.rows().map(|x| Cow::Owned(x.to_vec())).collect();

                    // print cutting rows
                    println!("Cutting Rows: {:?}", cutting_rows);

                    if files_to_merge.contains_key(&other_name) {
                        let val = files_to_merge.get_mut(&other_name).unwrap();
                        let rows = rows;
                        val.rows = rows;
                        val.is_main = is_main;
                    } else {
                        files_to_merge.insert(
                            name.replace("-MAIN", "").to_owned(),
                            File::new(other_name.clone(), "unknown".to_string(), rows, is_main),
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
    fn merge(&mut self) -> Result<MergeFiles> {
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

        let mut first_rows: Vec<DataType> = files_to_merge
            .iter()
            .next()
            .unwrap()
            .1
            .rows
            .first()
            .unwrap()
            .iter()
            .map(|data| data.to_owned())
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
        let mut values_rows: Vec<Vec<DataType>> = files_to_merge
            .iter()
            .enumerate()
            .flat_map(|(i, (_, inner_vec))| {
                let main_data: Vec<Vec<DataType>> = inner_vec
                    .rows
                    .iter()
                    .skip(1)
                    .enumerate()
                    .map(|(j, file)| {
                        let mut intro_headers: Vec<DataType> = vec![];
                        let cur_row_values: Vec<DataType> =
                            file.iter().map(|row_data| row_data.to_owned()).collect();

                        intro_headers
                            .push(DataType::DateTimeIso(inner_vec.last_modified.to_owned()));
                        intro_headers.push(DataType::Int((i + 1) as i64));
                        intro_headers.push(DataType::Int((acc_width + 1) as i64));
                        intro_headers.push(DataType::String(
                            (i + 1).to_string() + "-" + ((j) + 1).to_string().as_str(),
                        ));
                        intro_headers.push(DataType::String(inner_vec.name.replace("-MAIN", "")));

                        acc_width += 1;

                        [intro_headers, cur_row_values].concat()
                    })
                    .collect();

                // let directory = Path::new("text");

                // let file_path = directory.join(format!("{}.txt", &inner_vec.name));

                // // Open the file in write mode
                // let mut file = std::fs::File::create(&file_path).unwrap();

                // for row in &main_data {
                //     let line = row.join(",");
                //     writeln!(&mut file, "{}", line).unwrap();
                // }

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
        .map(|x| DataType::String(x.to_string()))
        .collect::<Vec<DataType>>();

        extra_headers.append(&mut first_rows);

        values_rows.insert(0, extra_headers);

        Ok(MergeFiles { rows: values_rows })
    }

    /// search and filter out the matched rows
    pub fn search(data: Vec<&[DataType]>, search: &[Search]) -> Result<SearchFiles> {
        // FIXME: I don't remember
        let headers = &data.clone()[0];
        let mut filtered_rows: Vec<Vec<DataType>> = vec![];

        for row in &data {
            let mut is_matched = false;
            for search in search {
                // TODO: see if this makes a problem with non-string types
                if row.contains(&DataType::String(search.data.to_owned())) {
                    let index = row
                        .iter()
                        .position(|x| x == &DataType::String(search.data.to_owned()))
                        .unwrap();

                    // data is matched, check other things now
                    is_matched = true;

                    // title
                    if let Some(title) = &search.title {
                        is_matched = headers[index] == DataType::String(title.to_owned());
                        // if the title doesn't match, then skip this iteration, it's not what we want
                        if !is_matched {
                            continue;
                        }
                    }

                    // intersections
                    if is_matched && !search.intersections.is_empty() {
                        println!("we are in intersections");

                        search.intersections.iter().for_each(|search| {
                            if row.contains(&DataType::String(search.clone().data)) {
                                println!("the row matched data: {:?}", &search.data);

                                let index = row
                                    .iter()
                                    .position(|x| x == &DataType::String(search.data.to_string()))
                                    .unwrap();

                                println!("index matched: {:?}", &index);

                                if let Some(title) = &search.title {
                                    println!("we be looking for title: {:?}", &title);
                                    is_matched = headers[index] == DataType::String(title.clone());

                                    if is_matched {
                                        println!("we found the title: {:?}", &title);
                                    } else {
                                        println!("we didn't find the title: {:?}", &title)
                                    }
                                }
                            } else {
                                is_matched = false;
                            }
                        })
                    }

                    // we push the row if it's matched and cut it
                    if is_matched {
                        filtered_rows.push(row.to_vec());
                    }
                }
            }
        }

        filtered_rows.insert(0, headers.to_vec());

        dbg!(&filtered_rows);

        Ok(SearchFiles {
            rows: filtered_rows,
        })
    }
}
