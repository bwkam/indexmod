use crate::error::Result;
use anyhow::Context;
use rust_xlsxwriter::Workbook;

pub struct MergeFiles {
    pub rows: Vec<Vec<String>>,
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
                worksheet.write_string(i as u32, j as u16, cell);
            }
        }

        let buf = workbook
            .save_to_buffer()
            .context("Failed to save workbook to buffer")?
            .to_vec();

        Ok(buf)
    }

    pub fn write_to_vec(&self) -> Vec<Vec<String>> {
        self.rows.clone()
    }
}
