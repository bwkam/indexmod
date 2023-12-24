use anyhow::Context;
use calamine::DataType;
use rust_xlsxwriter::{ExcelDateTime, Workbook};

use crate::error::Result;

#[derive(Clone, Debug)]
pub struct Search {
    pub data: String,
    pub title: Option<String>,
    pub intersections: Vec<Search>,
}

// TODO: Fix the visibility of structs like this
pub struct SearchFiles {
    pub rows: Vec<Vec<DataType>>,
}

impl SearchFiles {
    pub fn write_to_buffer(&self) -> Result<Vec<u8>> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        // write manually to the worksheet
        for (i, row) in self.rows.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                let _ = match cell {
                    DataType::String(s) => worksheet.write_string(i as u32, j as u16, s),
                    DataType::Int(n) => worksheet.write_number(i as u32, j as u16, *n as u32),
                    DataType::Float(f) => worksheet.write_number(i as u32, j as u16, *f),
                    // DataType::DateTimeIso(dt) => worksheet.write_datetime(
                    //     i as u32,
                    //     j as u16,
                    //     ExcelDateTime::parse_from_str(dt)
                    //         .context("Failed to parse date")
                    //         .unwrap(),
                    _ => worksheet.write_string(i as u32, j as u16, ""),
                };
            }
        }

        let buf = workbook
            .save_to_buffer()
            .context("Failed to save workbook to buffer")
            .unwrap()
            .to_vec();

        Ok(buf)
    }

    pub fn write_to_vec(&self) -> Vec<Vec<DataType>> {
        self.rows.clone()
    }
}
