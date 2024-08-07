use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Read, Seek};
use std::ops::Add;
use std::time::Instant;
use std::{io::Cursor, path::Path};

use crate::error::{Error, Result};
use crate::merge::MergeFiles;
use crate::reply::{MergedLocation, ReplyFile};

use anyhow::{anyhow, Context};
use axum::extract::Multipart;
use calamine::{Data, Dimensions, Range, Reader, Sheet, Xls, Xlsx};
use chrono::NaiveDateTime;
use itertools::Itertools;
use reply::{MergeType, ReplyFiles};
use search::{Search, SearchFiles};
use serde::Deserialize;
use tracing::{debug, info, trace, warn, Instrument};
use uuid::Uuid;

pub mod api;
pub mod error;
pub mod routes;

pub mod merge;
pub mod reply;

pub mod search;

//                 date    files   series  count    name
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
                        NaiveDateTime::parse_from_str(&v1.last_modified, "%Y/%m/%d %H:%M").unwrap();
                    let dt2 =
                        NaiveDateTime::parse_from_str(&v2.last_modified, "%Y/%m/%d %H:%M").unwrap();
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
        let mut first_rows: Vec<String> = vec![];

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

                let is_main = name == "main-file";

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
                                    Data::String(s) => s.to_owned(),
                                    Data::Float(s) => s.to_string(),
                                    Data::Int(s) => s.to_string(),
                                    Data::DateTime(s) => s.to_string(),
                                    Data::DateTimeIso(s) => s.to_string(),
                                    Data::Empty => "".to_string(),
                                    _ => "unknown".to_owned(),
                                })
                                .collect_vec()
                        })
                        .collect();

                    if is_main {
                        first_rows = rows[0].clone();
                    }

                    files.push(File::new(
                        other_name,
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

        Ok(MergeFiles { rows: values_rows })
    }

    // TODO: block workboots with more than a sheet!
    pub async fn reply_from_multipart(mut multipart: Multipart) -> Result<ReplyFiles> {
        let mut files: ReplyFiles = ReplyFiles::new(vec![]);
        let mut dates: Vec<String> = vec![];
        let mut rename: Vec<bool> = vec![];
        let mut cutting_rows: Vec<u32> = vec![];
        let mut sizes: Vec<u32> = vec![];
        let mut checked: Vec<bool> = vec![];
        let mut reply: Vec<bool> = vec![];

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

            if name == "checked[]" {
                if let Ok(checked_str) = String::from_utf8(bytes.to_vec()) {
                    if let Ok(checked_value) = checked_str.parse::<bool>() {
                        checked.push(checked_value);
                    }
                }
                trace!("checked: {:?}", checked);

                continue;
            }

            if name == "reply[]" {
                if let Ok(reply_str) = String::from_utf8(bytes.to_vec()) {
                    if let Ok(reply_value) = reply_str.parse::<bool>() {
                        reply.push(reply_value);
                    }
                }
                trace!("reply: {:?}", reply);

                continue;
            }

            if name == "cut-row[]" {
                if let Ok(cut_row_str) = String::from_utf8(bytes.to_vec()) {
                    if cut_row_str.is_empty() {
                        cutting_rows.push(0);
                    }
                    if let Ok(cut_row) = cut_row_str.parse::<u32>() {
                        cutting_rows.push(cut_row);
                    }
                }
                trace!("Cutting-rows: {:?}", cutting_rows);

                continue;
            }

            if name == "rename[]" {
                if let Ok(rename_str) = String::from_utf8(bytes.to_vec()) {
                    if let Ok(rename_value) = rename_str.parse::<bool>() {
                        rename.push(rename_value);
                    }
                }
                trace!("Rename: {:?}", rename);

                continue;
            }

            if name == "size[]" {
                if let Ok(size_str) = String::from_utf8(bytes.to_vec()) {
                    if let Ok(size) = size_str.parse::<u32>() {
                        sizes.push(size);
                    }
                }
                trace!("Size values: {:?}", sizes);

                continue;
            }

            if let Some(content_type) = content_type {
                let bytes = bytes.to_vec();
                let reader = Cursor::new(bytes);

                match content_type.as_str() {
                    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
                        let mut workbook: calamine::Xlsx<_> =
                            calamine::open_workbook_from_rs(reader).unwrap();

                        if workbook.worksheets().len() > 1 {
                            warn!("Has more than one sheet! Will only parse the first sheet.");
                            return Err(Error::SheetLimitExceeded);
                        } else {
                            debug!("Has only one sheet.")
                        }
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
                            trace!("Merged regions: {:?}", merged_regions);
                        }
                        process_workbook(&mut workbook, &other_name, &mut files, &merged_regions);
                    }
                    "application/vnd.ms-excel" => {
                        let mut workbook: calamine::Xls<_> =
                            calamine::open_workbook_from_rs(reader).unwrap();

                        if workbook.worksheets().len() > 1 {
                            warn!("Has more than one sheet! Will only parse the first sheet.");
                            return Err(Error::SheetLimitExceeded);
                        } else {
                            debug!("Has only one sheet.")
                        }
                        let mut merged_regions: Vec<Dimensions> = vec![];
                        println!("File name (xls): {:?}", &name);
                        if let Some(x) = workbook.worksheet_merge_cells_at(0) {
                            merged_regions = x;
                        }
                        process_workbook(&mut workbook, &other_name, &mut files, &merged_regions);
                    }
                    _ => {
                        // Handle other content types or errors
                    }
                }
                continue;
            }
        }

        files.data.iter_mut().enumerate().for_each(|(i, file)| {
            file.last_modified = dates[i].clone();
            file.cutting_rows = cutting_rows[i].clone();
            file.size = sizes[i].clone();
            file.rename = rename[i].clone();
            file.checked = checked[i].clone();
            file.reply = reply[i].clone();
        });

        dates.clear();
        cutting_rows.clear();
        sizes.clear();
        rename.clear();
        checked.clear();
        reply.clear();

        // dbg!(&files.data);

        files.data.retain(|file| file.checked == true);

        // assumption: there's only one sheet
        for file in &mut files.data {
            // do the cut
            let original_rows = file.rows.clone();
            file.rows = file.rows[(file.cutting_rows as usize)..].to_vec();
            if !file.merged_regions.is_empty() {
                // sort regions using rows from bottom to top
                file.merged_regions
                    .sort_by(|a, b| a.start.0.cmp(&b.start.0));

                trace!("Merged regions: {:?}", file.merged_regions);

                for merged_region in &mut file.merged_regions {
                    let merged_value = original_rows[(merged_region.start.0) as usize]
                        [(merged_region.start.1) as usize]
                        .to_owned();
                    let original_merge_regions = merged_region.clone();
                    // if it's a merged column
                    if merged_region.start.1 == merged_region.end.1 {
                        // trace!("row vals: {:?}", file.rows);
                        // trace!("row_data: {:?}", row_data);
                        trace!("col {:?}", merged_region);

                        let mut idx = 0;

                        // if it's entirely cut, then just ignore it
                        if merged_region.start.0 < file.cutting_rows
                            && merged_region.end.0 < file.cutting_rows
                        {
                            trace!("merge region cut, ignored");
                            continue;
                        }

                        trace!("cutting original merge regions");
                        // cut
                        merged_region.start.0 =
                            merged_region.start.0.saturating_sub(file.cutting_rows);

                        merged_region.end.0 = merged_region.end.0.saturating_sub(file.cutting_rows);

                        trace!("new merge regions: {:?}", merged_region);

                        // only one cell, so write that and skip this iteration
                        if merged_region.start.0 == merged_region.end.0 {
                            println!("single merge cell, writing normally");
                            file.rows[(merged_region.start.0) as usize]
                                [(merged_region.start.1) as usize] = merged_value;
                            continue;
                        }

                        // unmerge
                        if file.reply {
                            file.rows.iter_mut().for_each(|row| {
                                if idx >= merged_region.start.0 && idx <= merged_region.end.0 {
                                    row[(merged_region.start.1) as usize] = merged_value.clone();
                                }
                                idx += 1;
                            });
                        }

                        file.merged_locations.push(MergedLocation {
                            dimensions: (*merged_region, original_merge_regions),
                            data: merged_value,
                            variant: MergeType::Column,
                        });
                        // if it's a merged row
                    } else if merged_region.start.0 == merged_region.end.0 {
                        // trace!("row vals: {:?}", file.rows);
                        // trace!("row_data: {:?}", row_data);
                        let mut idx = 0;

                        // // check if it's cut, if yes then skip it
                        if merged_region.start.0 <= file.cutting_rows {
                            trace!("skipping: {:?}", merged_region);
                            continue;
                        }

                        trace!("row {:?}", merged_region);

                        merged_region.start.0 -= file.cutting_rows;
                        merged_region.end.0 -= file.cutting_rows;

                        // unmerge
                        if file.reply {
                            file.rows[(merged_region.start.0) as usize]
                                .iter_mut()
                                .for_each(|cell| {
                                    trace!("cell: {:?}", cell);
                                    if idx >= merged_region.start.1 && idx <= merged_region.end.1 {
                                        trace!(
                                            "changing {:?} to {:?}",
                                            *cell,
                                            merged_value.clone()
                                        );
                                        *cell = merged_value.clone();
                                    } else {
                                        trace!("not within range");
                                    }
                                    idx += 1;
                                });
                        }

                        file.merged_locations.push(MergedLocation {
                            dimensions: (*merged_region, original_merge_regions),
                            data: merged_value,
                            variant: MergeType::Row,
                        });
                    }
                }
            }
        }

        Ok(files)
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
                // TODO: we know the type, so use the static alternative from calamine
                let mut workbook = calamine::open_workbook_auto_from_rs(reader)
                    .context("error opening workbook")?;

                // let mut workbook: Xls<Cursor<Vec<u8>>> =
                //     calamine::open_workbook_from_rs(reader).context("error opening workbook")?;

                debug!("File name (excel): {:?}", &name);

                if workbook.worksheets().len() > 1 {
                    warn!("Has more than one sheet! Will only parse the first sheet.");
                } else {
                    debug!("Has only one sheet.")
                }

                debug!("Parsing a single-sheet file");

                if let Some(range) = workbook.worksheet_range_at(0) {
                    let sheet = range.context("error getting range")?;

                    let rows = sheet_to_rows(sheet);

                    info!("Parsing rows");

                    info!("Finished. Pushing the file");

                    files.push(File::new(
                        other_name,
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

        let filtered_rows = search_from_files(&files, &conditions);

        let total_rows = filtered_rows.0.len();
        info!("Total rows: {:?}", total_rows);

        info!("Starting to write.");

        Ok(SearchFiles {
            rows: filtered_rows,
            conditions: conditions.conditions,
        })
    }
}

fn search_from_files(files: &[File], conditions: &Conditions) -> (Vec<Vec<String>>, Vec<String>) {
    let mut filtered_files: Vec<File> = vec![];
    let mut headers: Vec<String> = vec![];
    let mut filtered_files_title_bars: Vec<(usize, Vec<String>)> = vec![];

    let mut total_rows_count = 0;
    let mut total_matched_files_count = 0;

    info!("Start searching.");

    let info = calc_file_row_num_infos(files);

    for (i, file) in files.iter().enumerate() {
        let instant = Instant::now();
        let mut is_matched;
        let mut new_file_rows: Vec<Vec<String>> = vec![];
        let mut points: usize = 0;

        for search in &conditions.conditions {
            let current_file_rows = &file.rows;

            headers = current_file_rows.first().unwrap().clone();

            for (j, row) in current_file_rows.iter().enumerate() {
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
                        if row.contains(&search.data) {
                            let index = row.iter().position(|x| x == &search.data).unwrap();

                            if let Some(title) = &search.title {
                                is_matched = headers[index] == *title;
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
                    let row_num_info = info.get(i).unwrap().get(j).unwrap();

                    let mut new_row = vec![
                        row_num_info.0.clone(),
                        (total_matched_files_count + 1).to_string(),
                        (total_rows_count + 1).to_string(),
                        row_num_info.3.to_string(),
                        row_num_info.4.clone(),
                    ];

                    new_row.extend_from_slice(row);

                    new_file_rows.push(new_row);

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

        debug!("iteration duration: {:?}", instant.elapsed());
    }

    info!("Searching finished.");
    info!("Calculating the main title bar");

    let mut headers = merge_title_bars(&filtered_files_title_bars);
    info!("Finished calculating the main title bar.");

    info!("Adjusting the rows.");

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
                .map(|(i, cell)| (file_header.get(i).unwrap(), cell))
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
                    new_cells.push(field.to_string());
                } else {
                    let empty = "".to_string();
                    new_cells.push(empty);
                }
            });

            *cells = new_cells;
        });
    });

    info!("Finished adjusting the rows.");

    let mut final_rows = filtered_files
        .into_iter()
        .flat_map(|file| file.rows)
        .collect_vec();

    headers.0.dedup();

    final_rows.insert(0, headers.0);

    (final_rows, headers.1)
}

fn sheet_to_rows(sheet: Range<Data>) -> Vec<Vec<String>> {
    let rows: Vec<Vec<String>> = sheet
        .rows()
        .map(|row| {
            row.iter()
                .map(|cell| match cell {
                    Data::String(s) => s.to_owned(),
                    Data::Float(s) => s.to_string(),
                    Data::Int(s) => s.to_string(),
                    Data::DateTime(s) => s.to_string(),
                    Data::DateTimeIso(s) => s.to_string(),
                    Data::Empty => "".to_string(),
                    _ => "unkown".to_owned(),
                })
                .collect_vec()
        })
        .collect();

    rows
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

fn get_file_extension(filename: &str) -> Option<&str> {
    filename.rfind('.').map(|index| &filename[index + 1..])
}

fn calc_file_row_num_infos(files: &[File]) -> Vec<FileRowNumInfo> {
    let mut total_rows_count = 0;
    let file_row_num_infos: Vec<FileRowNumInfo> = files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let mut file_row_num_info: FileRowNumInfo = vec![];

            file.rows.iter().enumerate().for_each(|(j, _)| {
                file_row_num_info.push((
                    file.last_modified.clone(),
                    (i + 1).to_string(),
                    total_rows_count.to_string(),
                    format!("{}-{}", i + 1, j + 1),
                    file.name.clone(),
                ));

                total_rows_count += 1;
            });

            file_row_num_info
        })
        .collect();

    file_row_num_infos
}

fn merge_title_bars(title_bars: &[(usize, Vec<String>)]) -> (Vec<String>, Vec<String>) {
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

    // TODO test
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

    // TODO: turn `main_points_bar` and `title_bar_rows` to hashsets
    //  this is important so we can avoid cloning, which is potentially expensive
    let intersections = main_points_bar
        .clone()
        .into_iter()
        .collect::<HashSet<String>>()
        .intersection(&title_bar_rows.into_iter().collect::<HashSet<String>>())
        .map(|x| x.to_string())
        .collect_vec();

    (main_bar, intersections)
}

fn process_workbook<R, RS>(
    workbook: &mut R,
    // name: &str,
    other_name: &str,
    files: &mut ReplyFiles,
    merged_regions: &Vec<Dimensions>,
) where
    R: calamine::Reader<RS>,
    RS: Read + Seek,
{
    if let Ok(range) = workbook.worksheet_range_at(0).unwrap() {
        let sheet_name = &workbook.worksheets()[0];

        let rows: Vec<Vec<String>> = range
            .rows()
            .map(|row| {
                // trace!("row: {:?}", row);
                row.iter()
                    .map(|cell| match cell {
                        Data::String(s) => s.to_owned(),
                        Data::Float(s) => s.to_string(),
                        Data::Int(s) => s.to_string(),
                        Data::DateTime(s) => s.to_string(),
                        Data::DateTimeIso(s) => s.to_string(),
                        Data::Empty => "".to_string(),
                        _ => "unkown".to_owned(),
                    })
                    .collect_vec()
            })
            .collect();

        let other_name_clone = other_name.to_owned();
        let ext = get_file_extension(&other_name_clone).unwrap();

        files.data.push(ReplyFile::new(
            other_name.to_owned(),
            "unknown".to_string(),
            rows,
            ext.to_string(),
            0,
            0,
            merged_regions.to_owned(),
            vec![],
            vec![],
            false,
            sheet_name.0.to_string(),
            false,
            false,
        ));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_find_dup_indices() {
        assert_eq!(find_dup_indices("C", &["A", "B", "C", "C"]), vec![2, 3]);
    }
}
