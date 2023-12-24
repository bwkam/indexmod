use std::{io::Cursor, path::Path};

use crate::error::{Error, Result};
use crate::search::{Search, SearchFiles};
use crate::{File, SortBy};
use anyhow::Context;
use axum::extract::Multipart;
use calamine::{DataType, Reader};
use chrono::NaiveDateTime;
use indexmap::IndexMap;
use rust_xlsxwriter::{ExcelDateTime, Workbook};

pub struct MergeFiles {
    pub rows: Vec<Vec<DataType>>,
}

// TODO: write a trait instead for both search and merge
impl MergeFiles {
    /// save the merged file to a buffer
    pub fn write_to_buffer(&mut self) -> Result<Vec<u8>> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        // write manually to the worksheet
        for (i, row) in self.rows.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                let _ = match cell {
                    DataType::String(s) => worksheet.write_string(i as u32, j as u16, s),
                    DataType::Int(n) => worksheet.write_number(i as u32, j as u16, *n as u32),
                    DataType::Float(f) => worksheet.write_number(i as u32, j as u16, *f),
                    DataType::DateTimeIso(dt) => worksheet.write_datetime(
                        i as u32,
                        j as u16,
                        ExcelDateTime::parse_from_str(dt).context("Failed to parse date")?,
                    ),
                    _ => worksheet.write_string(i as u32, j as u16, ""),
                };
            }
        }

        let buf = workbook
            .save_to_buffer()
            .context("Failed to save workbook to buffer")?
            .to_vec();

        Ok(buf)
    }

    pub fn write_to_vec(&self) -> Vec<Vec<DataType>> {
        self.rows.clone()
    }
}
