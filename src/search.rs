use anyhow::Context;
use calamine::DataType;
use rust_xlsxwriter::Workbook;
use serde::Deserialize;

use crate::error::Result;

#[derive(Clone, Debug, Deserialize)]
pub struct Search {
    pub data: String,
    pub title: Option<String>,
    pub intersections: Vec<Search>,
}

// TODO: Fix the visibility of structs like this
pub struct SearchFiles {
    pub rows: Vec<Vec<String>>,
    pub conditions: Vec<Search>,
}

impl SearchFiles {
    pub fn write_to_buffer(&self) -> Result<Vec<u8>> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        // write manually to the worksheet
        for (i, row) in self.rows.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                worksheet.write_string(i as u32, j as u16, cell);
            }
        }

        let buf = workbook
            .save_to_buffer()
            .context("Failed to save workbook to buffer")
            .unwrap()
            .to_vec();

        Ok(buf)
    }

    pub fn write_to_vec(&self) -> Vec<Vec<String>> {
        self.rows.clone()
    }
}
