use crate::error::Result;
use anyhow::Context;
use calamine::Dimensions;
use std::io::{Cursor, Write};

use rust_xlsxwriter::{Format, Workbook};
use zip::{write::SimpleFileOptions, ZipWriter};

pub struct ReplyFiles {
    pub data: Vec<ReplyFile>,
}

#[derive(Debug, Clone)]
pub struct MergedLocation {
    pub dimensions: Dimensions,
    pub data: String,
}

#[derive(Debug)]
pub struct ReplyFile {
    pub name: String,
    pub last_modified: String,
    pub rows: Vec<Vec<String>>,
    pub ext: String,
    pub size: u32,
    pub cutting_rows: u32,
    pub merged_regions: Vec<Dimensions>,
    pub location_sheet_rows: Vec<Vec<String>>,
    pub merged_locations: Vec<MergedLocation>,
}

impl ReplyFiles {
    pub fn new(data: Vec<ReplyFile>) -> Self {
        ReplyFiles { data }
    }
}

impl ReplyFile {
    pub fn new(
        name: String,
        last_modified: String,
        rows: Vec<Vec<String>>,
        ext: String,
        size: u32,
        cutting_rows: u32,
        merged_regions: Vec<Dimensions>,
        location_sheet_rows: Vec<Vec<String>>,
        merged_locations: Vec<MergedLocation>,
    ) -> Self {
        ReplyFile {
            last_modified,
            name,
            rows,
            ext,
            size,
            cutting_rows,
            merged_regions,
            location_sheet_rows,
            merged_locations,
        }
    }
}

impl ReplyFiles {
    //// save the merged file to a buffer
    pub fn write_to_buffer(&mut self, single: bool, reply: bool) -> Result<Vec<u8>> {
        if !single {
            let mut buffer: Vec<u8> = Vec::new();
            let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
            let options = SimpleFileOptions::default();

            for file in &self.data {
                // write the actual file
                let mut workbook = Workbook::new();
                let worksheet = workbook.add_worksheet();

                // write manually to the worksheet
                for (i, row) in file.rows.iter().enumerate() {
                    for (j, cell) in row.iter().enumerate() {
                        worksheet
                            .write_string(i as u32, j as u16, cell)
                            .context("error writing to the new worksheet")?;
                    }
                }

                if reply {
                    // write the location sheet
                    let loc_worksheet = workbook.add_worksheet();

                    // write the header
                    loc_worksheet
                        .write_row(0, 0, file.rows[0].to_owned())
                        .context("error writing header")?;

                    // write the merged regions
                    file.merged_locations.iter().for_each(|region| {
                        loc_worksheet
                            .merge_range(
                                region.dimensions.start.0,
                                (region.dimensions.start.1) as u16,
                                region.dimensions.end.0 - file.cutting_rows,
                                (region.dimensions.end.1) as u16,
                                region.data.as_str(),
                                &Format::new(),
                            )
                            .context("error writing merged region")
                            .unwrap();
                    });
                } else if !file.merged_regions.is_empty() {
                    file.merged_locations.iter().for_each(|location| {
                        worksheet
                            .merge_range(
                                location.dimensions.start.0,
                                (location.dimensions.start.1) as u16,
                                location.dimensions.end.0 - file.cutting_rows,
                                (location.dimensions.end.1) as u16,
                                location.data.as_str(),
                                &Format::new(),
                            )
                            .context("error writing merged region")
                            .unwrap();
                    });
                }

                let buf = workbook
                    .save_to_buffer()
                    .context("Failed to save workbook to buffer")?
                    .to_vec();

                zip.start_file(file.name.to_owned(), options)
                    .context("Error starting file")?;
                zip.write(buf.as_slice())
                    .context("Error writing excel file to the zip")?;
            }

            zip.finish().unwrap();
            return Ok(buffer);
        } else {
            let mut workbook = Workbook::new();
            let worksheet = workbook.add_worksheet();

            // write manually to the worksheet
            for (i, row) in self.data[0].rows.iter().enumerate() {
                for (j, cell) in row.iter().enumerate() {
                    worksheet
                        .write_string(i as u32, j as u16, cell)
                        .context("error writing to the new worksheet")?;
                }
            }

            // write the location sheet
            if reply {
                let loc_worksheet = workbook.add_worksheet();
                // write the header
                loc_worksheet
                    .write_row(0, 0, self.data[0].rows[0].to_owned())
                    .context("error writing header")?;

                // write the merged regions
                self.data[0].merged_locations.iter().for_each(|region| {
                    loc_worksheet
                        .merge_range(
                            region.dimensions.start.0,
                            (region.dimensions.start.1) as u16,
                            region.dimensions.end.0 - self.data[0].cutting_rows,
                            (region.dimensions.end.1) as u16,
                            region.data.as_str(),
                            &Format::new(),
                        )
                        .context("error writing merged region")
                        .unwrap();
                });
            } else if !self.data[0].merged_regions.is_empty() {
                self.data[0].merged_locations.iter().for_each(|location| {
                    worksheet
                        .merge_range(
                            location.dimensions.start.0,
                            (location.dimensions.start.1) as u16,
                            location.dimensions.end.0 - self.data[0].cutting_rows,
                            (location.dimensions.end.1) as u16,
                            location.data.as_str(),
                            &Format::new(),
                        )
                        .context("error writing merged region")
                        .unwrap();
                });
            }

            let buf = workbook
                .save_to_buffer()
                .context("Failed to save workbook to buffer")?
                .to_vec();

            return Ok(buf);
        }
    }
}
