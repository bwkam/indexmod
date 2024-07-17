use crate::error::Result;
use anyhow::Context;
use calamine::Dimensions;
use std::{
    io::{Cursor, Write},
    path::PathBuf,
};
use tracing::{info, trace};

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
    /// (cut, original)
    pub dimensions: (Dimensions, Dimensions),
    pub data: String,
    pub variant: MergeType,
}

#[derive(Debug)]
pub struct ReplyFile {
    pub name: PathBuf,
    pub last_modified: String,
    pub rows: Vec<Vec<String>>,
    pub ext: String,
    pub size: u32,
    pub cutting_rows: u32,
    pub merged_regions: Vec<Dimensions>,
    pub location_sheet_rows: Vec<Vec<String>>,
    pub merged_locations: Vec<MergedLocation>,
    pub rename: bool,
    pub sheet_name: String,
    pub checked: bool,
    pub reply: bool,
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
        rename: bool,
        sheet_name: String,
        checked: bool,
        reply: bool,
    ) -> Self {
        ReplyFile {
            last_modified,
            name: name.into(),
            rows,
            ext,
            size,
            cutting_rows,
            merged_regions,
            location_sheet_rows,
            merged_locations,
            rename,
            sheet_name,
            checked,
            reply,
        }
    }
}

impl ReplyFiles {
    //// save the merged file to a buffer
    pub fn write_to_buffer(&mut self, single: bool) -> Result<Vec<u8>> {
        if !single {
            let mut buffer: Vec<u8> = Vec::new();
            let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

            for file in &self.data {
                // write the actual file
                let mut workbook = Workbook::new();
                let mut worksheet = workbook.add_worksheet();

                if file.rename {
                    worksheet = worksheet
                        .set_name("Original")
                        .context("error setting name of original sheet")?;
                } else {
                    worksheet = worksheet
                        .set_name(&file.sheet_name)
                        .context("error setting name of original sheet")?;
                }

                // write manually to the worksheet
                // TODO: use the matrix function to be more concise
                for (i, row) in file.rows.iter().enumerate() {
                    for (j, cell) in row.iter().enumerate() {
                        worksheet
                            .write_string(i as u32, j as u16, cell)
                            .context("error writing to the new worksheet")?;
                    }
                }

                if file.reply && !file.merged_locations.is_empty() {
                    Self::write_loc_sheet(&mut workbook, &file.rows, &file.merged_locations)?;
                } else if !file.merged_locations.is_empty() {
                    for location in &file.merged_locations {
                        worksheet
                            .merge_range(
                                location.dimensions.0.start.0,
                                (location.dimensions.0.start.1) as u16,
                                location.dimensions.0.end.0,
                                (location.dimensions.0.end.1) as u16,
                                location.data.as_str(),
                                &Format::new(),
                            )
                            .context("error writing merged region")?;
                    }
                }

                let buf = workbook
                    .save_to_buffer()
                    .context("Failed to save workbook to buffer")?
                    .to_vec();

                zip.start_file(file.name.to_string_lossy(), options)
                    .context("error 0.starting file")?;
                zip.write(buf.as_slice())
                    .context("error writing excel file to the zip")?;
            }

            zip.finish().unwrap();
            return Ok(buffer);
        } else {
            let mut workbook = Workbook::new();
            let mut worksheet = workbook.add_worksheet();
            let file = &self.data[0];

            if file.rename {
                worksheet = worksheet
                    .set_name("Original")
                    .context("error setting name of original sheet")?;
            } else {
                worksheet = worksheet
                    .set_name(&file.sheet_name)
                    .context("error setting name of original sheet")?;
            }

            // write manually to the worksheet
            for (i, row) in self.data[0].rows.iter().enumerate() {
                for (j, cell) in row.iter().enumerate() {
                    worksheet
                        .write_string(i as u32, j as u16, cell)
                        .context("error writing to the new worksheet")?;
                }
            }

            // write the location sheet
            if file.reply && !file.merged_locations.is_empty() {
                Self::write_loc_sheet(&mut workbook, &file.rows, &file.merged_locations)?;
            } else if !file.merged_locations.is_empty() {
                for location in file.merged_locations.iter() {
                    worksheet
                        .merge_range(
                            location.dimensions.0.start.0,
                            (location.dimensions.0.start.1) as u16,
                            location.dimensions.0.end.0,
                            (location.dimensions.0.end.1) as u16,
                            location.data.as_str(),
                            &Format::new(),
                        )
                        // .context("error writing merged region")?;
                        .unwrap();
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
        let sheet = sheet
            .set_name("Location")
            .context("error setting name of loc sheet")?;
        let header = &data[0];

        // write the top header
        sheet
            .write_row(0, 0, header)
            .context("error writing header")?;

        for location in merged_locations {
            sheet
                .merge_range(
                    location.dimensions.0.start.0,
                    (location.dimensions.0.start.1) as u16,
                    location.dimensions.0.end.0,
                    (location.dimensions.0.end.1) as u16,
                    location.data.as_str(),
                    &Format::new(),
                )
                .unwrap();
        }

        Ok(())
    }
}
