use std::{io::Cursor, path::Path};

use crate::error::{Error, Result};
use crate::merge::MergeFiles;

use axum::extract::Multipart;
use calamine::{DataType, Reader};
use chrono::NaiveDateTime;
use itertools::Itertools;
use search::{Search, SearchFiles};
use serde::Deserialize;
use uuid::Uuid;

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

#[derive(Clone, Debug, Deserialize)]
pub struct Conditions {
    pub conditions: Vec<Search>,
}

// TODO: Refactor the vector `clone`s to `cloned`s
#[derive(Clone, Debug)]
/// A file is a struct that represents a single file to be merged
pub struct File {
    pub last_modified: String,
    pub name: String,
    pub rows: Vec<Vec<String>>,
    pub is_main: bool,
    pub id: uuid::Uuid,
}

impl File {
    /// Creates a new file struct
    pub fn new(
        name: String,
        last_modified: String,
        rows: Vec<Vec<String>>,
        is_main: bool,
        id: Uuid,
    ) -> Self {
        File {
            last_modified,
            rows,
            name,
            is_main,
            id,
        }
    }
}

/// A files map is a struct that represents an map of files to be merged, and some other options
pub struct FilesMap {
    pub files: Vec<File>,
    pub sort_by_date: bool,
    pub sort_by_file: bool,
    pub cutting_rows: usize,
}

impl FilesMap {
    /// sort based on date or file name
    fn sort(files: &mut [File], sort: SortBy) {
        match sort {
            SortBy::Date => {
                print!("Sorting by date...");
                files.sort_by(|v1, v2| {
                    let dt1 =
                        NaiveDateTime::parse_from_str(&v1.last_modified, "%Y %m %d %H%M").unwrap();
                    let dt2 =
                        NaiveDateTime::parse_from_str(&v2.last_modified, "%Y %m %d %H%M").unwrap();
                    dt1.cmp(&dt2)
                });
            }

            SortBy::File => {
                println!("Sorting by file...");
                files.sort_by(|b, d| {
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

    /// merge files
    pub async fn merge_from_multipart(mut multipart: Multipart) -> Result<MergeFiles> {
        let mut files: Vec<File> = vec![];
        let mut dates: Vec<String> = vec![];

        let mut cutting_rows: usize = 0;
        let mut sort_by_date: bool = false;
        let mut sort_by_file: bool = false;

        while let Some(field) = multipart.next_field().await.unwrap() {
            let content_type = field.content_type().map(str::to_owned);

            let name = field.name().unwrap_or("unknown").to_owned();
            let other_name = field.file_name().unwrap_or("unknown").to_owned();
            let bytes = field.bytes().await.unwrap();

            if name.starts_with("sort-by") {
                if name == "sort-by-date" {
                    let val = String::from_utf8(bytes.to_vec())
                        .unwrap()
                        .parse::<bool>()
                        .unwrap();

                    if val {
                        sort_by_date = true;
                    }
                } else if name == "sort-by-file" {
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

            if name == "last-mod[]" {
                let date = String::from_utf8(bytes.to_vec()).unwrap();

                dates.push(date);

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
                let is_main = false;
                let reader = Cursor::new(bytes);
                let mut workbook = calamine::open_workbook_auto_from_rs(reader).unwrap();

                println!("File name (excel): {:?}", &name);

                if workbook.worksheets().len() > 1 {
                    return Err(Error::SheetLimitExceeded);
                }

                if let Some(range) = workbook.worksheet_range_at(0) {
                    let sheet = range.unwrap();
                    let rows: Vec<Vec<String>> = sheet
                        .rows()
                        .map(|row| {
                            row.iter()
                                .map(|cell| match cell {
                                    DataType::String(s) => s.to_owned(),
                                    _ => "empty".to_owned(),
                                })
                                .collect_vec()
                        })
                        .collect();

                    files.push(File::new(
                        other_name.clone(),
                        "unknown".to_string(),
                        rows,
                        is_main,
                        Uuid::new_v4(),
                    ));
                }

                continue;
            }
        }

        files.sort_by_key(|file| !file.is_main);

        files.iter_mut().enumerate().for_each(|(i, file)| {
            file.last_modified = dates[i].clone();
        });

        dates.clear();

        println!("Files: {:?}", &files);

        if sort_by_date {
            FilesMap::sort(&mut files, SortBy::Date);
        } else if sort_by_file {
            FilesMap::sort(&mut files, SortBy::File);
        }

        // sorting that will run anyways
        files.sort_by_key(|file| !file.is_main);

        files.iter().for_each(|v| {
            println!(
                "Name: {:?}, rows: {:?}, is_main: {:?}, date_modified: {:?}",
                v.name,
                v.rows.len(),
                v.is_main,
                v.last_modified
            );
        });

        println!("Merging files...");

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
            files.iter_mut().filter(|x| !x.is_main).for_each(|v| {
                v.rows = v.rows.drain((cutting_rows - 1)..).collect();
            });
        }

        let mut acc_width = 0;
        let mut values_rows: Vec<Vec<String>> = files
            .iter()
            .enumerate()
            .flat_map(|(i, inner_vec)| {
                let main_data: Vec<Vec<String>> = inner_vec
                    .rows
                    .iter()
                    .skip(1)
                    .enumerate()
                    .map(|(j, file)| {
                        let mut intro_headers: Vec<String> = vec![];
                        let cur_row_values: Vec<String> =
                            file.iter().map(|row_data| row_data.to_owned()).collect();

                        intro_headers.push(inner_vec.last_modified.to_owned());
                        intro_headers.push((i + 1).to_string());
                        intro_headers.push((acc_width + 1).to_string());
                        intro_headers
                            .push((i + 1).to_string() + "-" + ((j) + 1).to_string().as_str());
                        intro_headers.push(inner_vec.name.replace("-MAIN", ""));

                        acc_width += 1;

                        [intro_headers, cur_row_values].concat()
                    })
                    .collect();

                main_data
            })
            .collect();

        // modify the headers
        let extra_headers = [
            "Date Modified",
            "Number of Files",
            "Series Number",
            "Count Number",
            "File Name",
        ]
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>();

        // extra_headers.append(&mut first_rows);

        values_rows.insert(0, extra_headers);

        Ok(MergeFiles { rows: values_rows })
    }

    /// search and filter out the matched rows
    pub async fn search_from_multipart(mut multipart: Multipart) -> Result<SearchFiles> {
        let mut files: Vec<File> = vec![];
        let mut dates: Vec<String> = vec![];
        let mut conditions: Conditions = Conditions { conditions: vec![] };

        // fetch the results from the multipart form
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

                if workbook.worksheets().len() > 1 {
                    return Err(Error::SheetLimitExceeded);
                }

                if name == "main-file" {
                    is_main = true;
                }

                if let Some(range) = workbook.worksheet_range_at(0) {
                    let sheet = range.unwrap();

                    let rows: Vec<Vec<String>> = sheet
                        .rows()
                        .map(|row| {
                            row.iter()
                                .map(|cell| match cell {
                                    DataType::String(s) => s.to_owned(),
                                    DataType::Int(s) => s.to_string(),
                                    DataType::Float(s) => s.to_string(),
                                    DataType::Empty => " ".to_string(),
                                    _ => "empty".to_owned(),
                                })
                                .collect_vec()
                        })
                        .collect();

                    files.push(File::new(
                        other_name.clone(),
                        "unknown".to_string(),
                        rows,
                        is_main,
                        Uuid::new_v4(),
                    ));
                }

                continue;
            }

            if name == "conditions" {
                conditions = serde_json::from_slice(bytes.as_ref()).unwrap();

                println!("Conditions: {:?}", &conditions);

                continue;
            }
        }

        // set the right dates, index based
        files.iter_mut().enumerate().for_each(|(i, file)| {
            file.last_modified = dates[i].clone();
        });

        dates.clear();

        files.iter().for_each(|v| {
            println!(
                "Name: {:?}, rows: {:?}, is_main: {:?}, date_modified: {:?}",
                v.name,
                v.rows.len(),
                v.is_main,
                v.last_modified
            );
        });

        println!("Merging files...");


        // merging the files's rows into one big vec
        // let mut values_rows: Vec<Vec<DataType>> = files
        //     .iter()
        //     .enumerate()
        //     .flat_map(|(i, file)| {
        //         let main_data: Vec<Vec<DataType>> = file
        //             .rows
        //             .iter()
        //             .skip(1)
        //             .enumerate()
        //             .map(|(j, rows)| {
        //                 let mut intro_headers: Vec<DataType> = vec![];
        //                 let cur_row_values: Vec<DataType> =
        //                     rows.iter().map(|row_data| row_data.to_owned()).collect();
        //
        //                 intro_headers.push(DataType::String(file.last_modified.to_owned()));
        //                 intro_headers.push(DataType::Int((i + 1) as i64));
        //                 intro_headers.push(DataType::Int((acc_width + 1) as i64));
        //                 intro_headers.push(DataType::String(
        //                     (i + 1).to_string() + "-" + ((j) + 1).to_string().as_str(),
        //                 ));
        //                 intro_headers.push(DataType::String(file.name.replace("-MAIN", "")));
        //
        //                 acc_width += 1;
        //
        //                 [intro_headers, cur_row_values].concat()
        //             })
        //             .collect();
        //
        //         main_data
        //     })
        //     .collect();

        // modify the headers
        let _intro_headers = [
            "Date Modified",
            "Number of Files",
            "Series Number",
            "Count Number",
            "File Name",
        ]
        .iter()
        .map(|x| DataType::String(x.to_string()))
        .collect::<Vec<DataType>>();

        // filtered_rows.insert(0, headers.to_vec());

        let filtered_rows = search_from_files(files, conditions.clone());

        Ok(SearchFiles {
            rows: filtered_rows,
            conditions: conditions.conditions,
        })
    }
}

fn search_from_files(files: Vec<File>, conditions: Conditions) -> Vec<Vec<String>> {
    let mut filtered_rows: Vec<Vec<String>> = vec![];
    let mut filtered_files: Vec<File> = vec![];

    let mut headers: Vec<String> = vec![];

    let mut filtered_files_title_bars: Vec<(usize, Vec<String>)> = vec![];
    let mut acc_width = 0;

    for (i, file) in &files.iter().enumerate().collect_vec() {
        let mut is_matched = false;
        let mut new_file_rows: Vec<Vec<String>> = vec![];
        let points_and_rows_cur_file: (u32, Vec<String>) = (0, vec![]);
        let mut points: usize = 0;

        for search in &conditions.conditions {
            let current_file_rows = &file.rows;

            headers = current_file_rows.to_owned().first().unwrap().clone();

            for (j, row) in current_file_rows.iter().enumerate().collect_vec() {
                acc_width += 1;
                let filtered_row = row.iter().filter(|x| **x == search.data).collect_vec();

                if filtered_row.is_empty() {
                    // if there are even no matches, then skip the next row
                    continue;
                }

                println!("we found a match: {:?}", &search.data);

                let dups = row
                    .iter()
                    .duplicates()
                    .filter(|x| **x == search.data)
                    .collect_vec();

                // TODO: handle duplicates, because it could match twice, and have wrong
                // title headers
                let index = row.iter().position(|x| *x == search.data).unwrap();

                // data is matched, check the title
                is_matched = true;

                // title
                if let Some(title) = &search.title {
                    if !title.is_empty() {
                        is_matched = headers[index] == *title;
                        // if the title doesn't match, then skip to the next cell, it's not what we want
                        if !is_matched {
                            continue;
                        }
                    }
                }

                // intersections
                if is_matched && !search.intersections.is_empty() {
                    println!("we are in intersections");
                    println!("intersections: {:?}", &search.intersections);

                    search.intersections.iter().for_each(|search| {
                        if row.contains(&search.clone().data) {
                            println!("the row matched data: {:?}", &search.data);

                            let index = row
                                .iter()
                                .position(|x| x == &search.data.to_string())
                                .unwrap();

                            println!("index matched: {:?}", &index);

                            if let Some(title) = &search.title {
                                println!("we be looking for title: {:?}", &title);
                                is_matched = headers[index] == title.clone();

                                if is_matched {
                                    println!("we found the title: {:?}", &title);
                                } else {
                                    println!("we didn't find the title: {:?}", &title)
                                }
                            }
                        } else {
                            println!("the row didn't match data: {:?}", &search.data);
                            is_matched = false;
                        }
                    })
                }

                // points handling

                // if we have duplicates, then add a point only for those whose title matches the
                // query title
                if !dups.is_empty() {
                    let idxs = find_dup_indices(dups[0], row);
                    println!("Duplicate we found {:?}", dups[0]);

                    for idx in idxs {
                        if let Some(title) = &search.title {
                            if !title.is_empty() {
                                println!("Comparing {:?} with {:?}", headers[idx], *title);
                                if headers[idx] == *title {
                                    points += 1;
                                }
                            } else {
                                points += 1;
                            }
                        }
                    }
                } else {
                    // otherwise, add a single point
                    points += 1
                }

                // we push the row if it's matched
                if is_matched {

                    let mut final_row = vec![];



                    let mut intro_headers = vec![];


                    intro_headers.push(file.last_modified.to_owned());
                    intro_headers.push((i + 1).to_string());
                    intro_headers.push((acc_width + 1).to_string());
                    intro_headers.push(
                        (i + 1).to_string() + "-" + ((j) + 1).to_string().as_str(),
                    );
                    intro_headers.push(file.name.replace("-MAIN", ""));

                    final_row = [intro_headers, row.to_vec()].concat();

                    new_file_rows.push(final_row.to_vec());
                    filtered_rows.push(final_row.to_vec());
                }
            }
        }

        filtered_files_title_bars.push((points, headers.clone()));

        filtered_files.push(File {
            rows: new_file_rows,
            is_main: false,
            name: file.name.clone(),
            last_modified: file.last_modified.clone(),
            id: file.id,
        });
    }

    let headers = merge_title_bars(filtered_files_title_bars);
    println!("final bar {:?}", headers);
    filtered_rows.insert(0, headers);

    filtered_rows
}

fn find_dup_indices(dup: &str, vec: &[impl AsRef<str>]) -> Vec<usize> {
    let mut indices = vec![];
    for (i, x) in vec.iter().enumerate() {
        if x.as_ref() == dup {
            indices.push(i)
        }
    }
    indices
}

fn merge_title_bars(title_bars: Vec<(usize, Vec<String>)>) -> Vec<String> {
    let highest_points_idx = title_bars.iter().map(|x| x.0).position_max().unwrap();
    let highest_points_bar = title_bars.get(highest_points_idx).unwrap();

    title_bars.iter().for_each(|x| println!("{:?}", x));

    println!("highest idx {:?}", highest_points_idx);
    println!("highest bar {:?}", highest_points_bar);

    let mut title_bars_clone = title_bars.clone();
    title_bars_clone.remove(highest_points_idx);

    let title_bar_rows = title_bars_clone
        .iter()
        .flat_map(|x| x.1.clone())
        .collect_vec();

    let result = highest_points_bar
        .1
        .clone()
        .into_iter()
        .chain(title_bar_rows.clone().into_iter())
        .unique()
        .collect_vec();

    println!(
        "chaining {:?} with {:?} into {:?}",
        highest_points_bar.1.clone(),
        title_bar_rows,
        result
    );

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_find_dup_indices() {
        assert_eq!(find_dup_indices("C", &["A", "B", "C", "C"]), vec![2, 3]);
    }

    #[test]
    fn test_merge_bars() {
        let vec1 = vec!["A", "B", "C"];
        let vec2 = vec!["A", "B", "C", "D", "E"];
        let vec3 = vec!["D", "E", "F"];

        // let result = vec1.into_iter().chain(vec2).collect::<HashSet<_>>().into_iter().collect_vec();
        let result = vec1.clone().into_iter().chain(vec2).unique().collect_vec();
        let result2 = vec1.into_iter().chain(vec3).unique().collect_vec();

        assert_eq!(
            result,
            ["A", "B", "C", "D", "E"]
                .iter()
                .map(|x| x.to_string())
                .collect_vec()
        );

        assert_eq!(
            result2,
            ["A", "B", "C", "D", "E", "F"]
                .iter()
                .map(|x| x.to_string())
                .collect_vec()
        )
    }
}
