use crate::error::Result;
use anyhow::Context;
use calamine::Dimensions;
use std::io::{Cursor, Write};
use tracing::trace;

use rust_xlsxwriter::{Format, Workbook};
use zip::{write::SimpleFileOptions, ZipWriter};

pub struct ReplyFiles {
    pub data: Vec<ReplyFile>,
}

#[derive(Debug, Clone)]
pub enum MergeType {
    Row,
    Column,
}

#[derive(Debug, Clone)]
pub struct MergedLocation {
    pub dimensions: Dimensions,
    pub data: String,
    pub variant: MergeType,
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
                // TODO: use the matrix function to be more concise
                for (i, row) in file.rows.iter().enumerate() {
                    for (j, cell) in row.iter().enumerate() {
                        worksheet
                            .write_string(i as u32, j as u16, cell)
                            .context("error writing to the new worksheet")?;
                    }
                }

                if reply {
                    Self::write_loc_sheet(&mut workbook, &file.rows, &file.merged_locations)?;
                } else if !file.merged_regions.is_empty() {
                    for location in &file.merged_locations {
                        match location.variant {
                            MergeType::Row => {
                                if location.dimensions.start.0 <= file.cutting_rows {
                                    continue;
                                } else {
                                    worksheet
                                        .merge_range(
                                            location.dimensions.start.0,
                                            (location.dimensions.start.1) as u16,
                                            location.dimensions.end.0,
                                            (location.dimensions.end.1) as u16,
                                            location.data.as_str(),
                                            &Format::new(),
                                        )
                                        .context("error writing merged region")?;
                                }
                            }

                            MergeType::Column => {
                                if location.dimensions.start.0 >= file.cutting_rows {
                                    worksheet
                                        .merge_range(
                                            location.dimensions.start.0,
                                            (location.dimensions.start.1) as u16,
                                            location.dimensions.end.0 - file.cutting_rows,
                                            (location.dimensions.end.1) as u16,
                                            location.data.as_str(),
                                            &Format::new(),
                                        )
                                        .context("error writing merged region")?;
                                }
                            }
                        }
                    }
                }

                let buf = workbook
                    .save_to_buffer()
                    .context("Failed to save workbook to buffer")?
                    .to_vec();

                zip.start_file(file.name.to_owned(), options)
                    .context("error starting file")?;
                zip.write(buf.as_slice())
                    .context("error writing excel file to the zip")?;
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
                Self::write_loc_sheet(
                    &mut workbook,
                    &self.data[0].rows,
                    &self.data[0].merged_locations,
                )?;
            } else if !self.data[0].merged_regions.is_empty() {
                for location in self.data[0].merged_locations.iter() {
                    match location.variant {
                        MergeType::Row => {
                            if location.dimensions.start.0 <= self.data[0].cutting_rows {
                                continue;
                            } else {
                                worksheet
                                    .merge_range(
                                        location.dimensions.start.0,
                                        (location.dimensions.start.1) as u16,
                                        location.dimensions.end.0,
                                        (location.dimensions.end.1) as u16,
                                        location.data.as_str(),
                                        &Format::new(),
                                    )
                                    .context("error writing merged region")?;
                            }
                        }

                        MergeType::Column => {
                            if location.dimensions.start.0 >= self.data[0].cutting_rows {
                                worksheet
                                    .merge_range(
                                        location.dimensions.start.0,
                                        (location.dimensions.start.1) as u16,
                                        location.dimensions.end.0 - self.data[0].cutting_rows,
                                        (location.dimensions.end.1) as u16,
                                        location.data.as_str(),
                                        &Format::new(),
                                    )
                                    .context("error writing merged region")?;
                            }
                        }
                    }
                    // FIXME: why errors and their BTs don't log?
                    // .merge_range(
                    //     0,
                    //     0,
                    //     0,
                    //     0,
                    //     location.data.as_str(),
                    //     &Format::new(),
                    // ).context("something bad happened")?;
                }
            }

            let buf = workbook
                .save_to_buffer()
                .context("Failed to save workbook to buffer")?
                .to_vec();

            return Ok(buf);
        }
    }

    pub fn write_loc_sheet(
        workbook: &mut Workbook,
        data: &[Vec<String>],
        merged_locations: &[MergedLocation],
    ) -> Result<()> {
        let sheet = workbook.add_worksheet();
        let header = &data[0];

        // write the top header
        sheet
            .write_row(0, 0, header)
            .context("error writing header")?;
        for location in merged_locations {
            sheet
                .merge_range(
                    location.dimensions.start.0,
                    (location.dimensions.start.1) as u16,
                    location.dimensions.end.0,
                    (location.dimensions.end.1) as u16,
                    location.data.as_str(),
                    &Format::new(),
                )
                .context("error writing merged region")?;
        }

        Ok(())
    }
}
