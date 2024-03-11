use anyhow::Context;

use itertools::Itertools;
use regex::Regex;
use rust_xlsxwriter::{Color, Format, Workbook};
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

        let default = Format::default();
        let red = Format::new().set_font_color(Color::Red);

        // write manually to the worksheet
        for (i, row) in self.rows.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                let mut segment = vec![];

                self.conditions.iter().for_each(|c| {
                    let pat = format!(r"(.*)(?<data>{})(.*)", c.data);
                    let re = Regex::new(pat.as_str()).unwrap();

                    if let Some(caps) = re.captures(cell) {
                        segment = caps
                            .iter()
                            .skip(1)
                            .filter(|c| !c.unwrap().is_empty())
                            .map(|cap| {
                                if cap.unwrap().as_str() == c.data {
                                    (&red, c.data.as_str())
                                } else {
                                    (&default, cap.unwrap().as_str())
                                }
                            })
                            .collect_vec();
                    } else {
                        segment = vec![(&default, cell)]
                    }
                });

                if cell.trim().is_empty() {
                    worksheet.write_string(i as u32, j as u16, cell).unwrap();
                    continue;
                }

                worksheet
                    .write_rich_string(i as u32, j as u16, segment.as_slice())
                    .unwrap();
            }
        }

        worksheet.autofit();

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
