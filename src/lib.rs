use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::{io::Cursor, path::Path};

use crate::error::{Error, Result};
use crate::merge::MergeFiles;

use anyhow::{anyhow, Context};
use axum::extract::Multipart;
use calamine::{DataType, Reader};
use chrono::NaiveDateTime;
use itertools::Itertools;
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rust_xlsxwriter::RowNum;
use search::{Search, SearchFiles};
use serde::Deserialize;
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

pub mod api;
pub mod error;
pub mod merge;
pub mod routes;
pub mod search;

//                 date    files   series  count  name
type RowNumInfo = (String, String, String, String, String);
type FileRowNumInfo = Vec<RowNumInfo>;

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
    /// first is main rows, second is the intro fields
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
                let date = String::from_utf8(bytes.to_vec()).context("error parsing date")?;

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
                let reader = Cursor::new(bytes);
                let mut workbook = calamine::open_workbook_auto_from_rs(reader)
                    .context("error opening workbook")?;

                debug!("File name (excel): {:?}", &name);

                if workbook.worksheets().len() > 1 {
                    warn!("Has more than one sheet! Will only parse the first sheet.");
                    // return Err(Error::SheetLimitExceeded);
                } else {
                    debug!("Has only one sheet.")
                }

                debug!("Parsing a single-sheet file");

                if let Some(range) = workbook.worksheet_range_at(0) {
                    let sheet = range.context("error getting range")?;

                    info!("Parsing rows");
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

                    info!("Finished. Pushing the file");

                    files.push(File::new(
                        other_name.clone(),
                        "unknown".to_string(),
                        rows,
                        false,
                        Uuid::new_v4(),
                    ));
                }

                continue;
            }

            if name == "conditions" {
                conditions =
                    serde_json::from_slice(bytes.as_ref()).context("error parsing conditions")?;

                debug!("Conditions: {:?}", &conditions);

                continue;
            }
        }

        if files.is_empty() {
            return Err(Error::Other(anyhow!("No files were found.")));
        }

        info!("Setting dates.");
        // set the right dates, index based
        files.iter_mut().enumerate().for_each(|(i, file)| {
            file.last_modified = dates[i].clone();
        });

        dates.clear();

        files.iter().for_each(|v| {
            trace!(
                "Name: {:?}, rows: {:?}, is_main: {:?}, date_modified: {:?}",
                v.name,
                v.rows.len(),
                v.is_main,
                v.last_modified
            );
        });

        info!("Merging files...");

        // modify the headers
        let intro_headers = [
            "Date Modified",
            "Number of Files",
            "Series Number",
            "Count Number",
            "File Name",
        ]
        .iter()
        .map(|x| x.to_string())
        .collect_vec();

        let mut filtered_rows = search_from_files(Arc::new(files), conditions.clone()).await;
        let header = filtered_rows.0.first().unwrap();

        let final_header = [intro_headers, header.clone()].concat();
        filtered_rows.0[0] = final_header;

        Ok(SearchFiles {
            rows: filtered_rows,
            conditions: conditions.conditions,
        })
    }
}

async fn search_from_files(
    files: Arc<Vec<File>>,
    conditions: Conditions,
) -> (Vec<Vec<String>>, Vec<String>) {
    let mut filtered_rows: Vec<Vec<String>> = vec![];
    let mut filtered_files: Vec<File> = vec![];
    let mut headers: Vec<String> = vec![];
    let mut filtered_files_title_bars: Vec<(usize, Vec<String>)> = vec![];
    let mut acc_width = 0;

    let mut total_rows_count = 0;
    // let file_row_num_infos: Vec<FileRowNumInfo>;

    let mut total_rows_count = 0;
    let mut total_matched_files_count = 0;

    for (i, file) in files.iter().enumerate().collect_vec() {
        let mut is_matched;
        let mut new_file_rows: Vec<Vec<String>> = vec![];
        let mut points: usize = 0;

        for search in &conditions.conditions {
            let current_file_rows = &file.rows;

            headers = current_file_rows.to_owned().first().unwrap().clone();

            for (j, row) in current_file_rows.iter().enumerate().collect_vec() {
                acc_width += 1;
                let filtered_row = row
                    .iter()
                    .filter(|x| x.contains(&search.data))
                    .collect_vec();

                if filtered_row.is_empty() {
                    // if there are even no matches, then skip the next row
                    continue;
                }

                let dups = row
                    .iter()
                    .duplicates()
                    .filter(|x| **x == search.data)
                    .collect_vec();

                let index = row.iter().position(|x| x.contains(&search.data)).unwrap();

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
                    search.intersections.iter().for_each(|search| {
                        if row.contains(&search.clone().data) {
                            let index = row
                                .iter()
                                .position(|x| x == &search.data.to_string())
                                .unwrap();

                            if let Some(title) = &search.title {
                                is_matched = headers[index] == title.clone();
                            }
                        } else {
                            is_matched = false;
                        }
                    })
                }

                // points handling
                // if we have duplicates, then add a point only for those whose title matches the
                // query title
                if !dups.is_empty() {
                    let idxs = find_dup_indices(dups[0], row);
                    for idx in idxs {
                        if let Some(title) = &search.title {
                            if !title.is_empty() {
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
                    let file_num_info = calc_file_row_num_infos(files.to_vec()).await;

                    let row_num_info = file_num_info.get(i).unwrap().get(j).unwrap();

                    let mut new_row = vec![
                        row_num_info.0.clone(),
                        (total_matched_files_count + 1).to_string(),
                        (total_rows_count + 1).to_string(),
                        row_num_info.3.to_string(),
                        row_num_info.4.clone(),
                    ];

                    new_row.extend_from_slice(row);

                    // TODO: filtered_rows isn't used anymore, so we don't need the clone?
                    new_file_rows.push(new_row.clone());
                    filtered_rows.push(new_row);

                    total_rows_count += 1;
                }
            }
        }

        if points > 0 {
            total_matched_files_count += 1;
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

    let mut headers = merge_title_bars(filtered_files_title_bars);
    let mut file_id_to_print = Uuid::new_v4();

    // adjust the rows because they are mispositioned at this point
    filtered_files.iter_mut().for_each(|file| {
        let file_header = files
            .iter()
            .find(|x| x.id == file.id)
            .unwrap()
            .rows
            .first()
            .unwrap();

        file.rows.iter_mut().for_each(|cells| {
            let (intro, cells_clone) = cells.split_at(5);

            let cells_and_headers: HashMap<_, _> = cells_clone
                .iter()
                .enumerate()
                .map(|(i, cell)| (file_header.get(i).unwrap().to_string(), cell))
                .collect();

            let mut new_cells = vec![];
            new_cells.extend_from_slice(intro);
            headers.0.iter().for_each(|header| {
                // before
                // A B C D   |   A D C
                // 1 2 3 4   |   1 4 3

                // after
                // A B C D   |   A B D C
                // 1 2 3 4   |   1   4 3

                // if the header has a field, put that, otherwise insert an empty
                if let Some(field) = cells_and_headers.get(header) {
                    if **field == "2" {
                        debug!("field {:?} belongs to the header {:?}", field, header);
                        file_id_to_print = file.id;
                    }
                    new_cells.push(field.to_string());
                } else {
                    let empty = "".to_string();
                    new_cells.push(empty);
                }
            });

            *cells = new_cells;
        });
    });

    // TODO: this is probably redundant
    let mut final_rows = filtered_files
        .iter()
        .flat_map(|file| file.rows.clone())
        .collect_vec();

    headers.0.dedup();
    final_rows.insert(0, headers.0);

    (final_rows, headers.1)
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

async fn calc_file_row_num_infos(files: Vec<File>) -> Vec<FileRowNumInfo> {
    let (send, recv) = tokio::sync::oneshot::channel();
    rayon::spawn(move || {
        let total_rows_count = AtomicUsize::new(0);
        let file_row_num_infos: Vec<FileRowNumInfo> = files
            .par_iter()
            .enumerate()
            .map(|(i, file)| {
                let mut file_row_num_info: FileRowNumInfo = vec![];

                file.rows.iter().enumerate().for_each(|(j, row)| {
                    file_row_num_info.push((
                        file.last_modified.clone(),
                        (i + 1).to_string(),
                        total_rows_count.fetch_add(1, Ordering::Relaxed).to_string(),
                        format!("{}-{}", i + 1, j + 1),
                        file.name.clone(),
                    ));
                });

                file_row_num_info
            })
            .collect();

        let _ = send.send(file_row_num_infos);
    });

    recv.await.expect("panic in rayon::spawn")
}

fn merge_title_bars(title_bars: Vec<(usize, Vec<String>)>) -> (Vec<String>, Vec<String>) {
    let mut title_bars_clone = title_bars
        .iter()
        .map(|x| {
            (
                x.0,
                x.1.iter()
                    .map(|x| x.chars().filter(|c| *c != '\n' && *c != '\r').collect())
                    .collect_vec(),
            )
        })
        .collect_vec();

    let main_points_idx = title_bars_clone.iter().map(|x| x.0).position_max().unwrap();
    let (_, main_points_bar) = title_bars_clone.remove(main_points_idx);

    let title_bar_rows = title_bars_clone
        .iter()
        .flat_map(|x| x.1.clone())
        .collect_vec();

    let main_bar = main_points_bar
        .clone()
        .into_iter()
        .chain(title_bar_rows.clone())
        .unique()
        .collect_vec();

    dbg!(&main_points_bar);
    dbg!(&title_bar_rows);

    // TODO: make this a hashset from the beginning
    let intersections = main_points_bar
        .clone()
        .into_iter()
        .collect::<HashSet<String>>()
        .intersection(&title_bar_rows.into_iter().collect::<HashSet<String>>())
        .map(|x| x.to_string())
        .collect_vec();

    (main_bar, intersections)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_find_dup_indices() {
        assert_eq!(find_dup_indices("C", &["A", "B", "C", "C"]), vec![2, 3]);
    }
}
