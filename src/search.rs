use anyhow::Context;

use itertools::Itertools;
use rust_xlsxwriter::{Color, Format, Workbook};
use serde::Deserialize;
use tracing::info;

use crate::error::Result;

#[derive(Clone, Debug, Deserialize)]
pub struct Search {
    pub data: String,
    pub title: Option<String>,
    pub intersections: Vec<Search>,
}

// TODO: Fix the visibility of structs like this
pub struct SearchFiles {
    pub rows: (Vec<Vec<String>>, Vec<String>),
    pub conditions: Vec<Search>,
}

impl SearchFiles {
    pub fn write_to_buffer(&mut self) -> Result<Vec<u8>> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        let default = Format::default();
        let red = Format::new().set_font_color(Color::Red);
        let pink_bg = Format::new().set_background_color(Color::Pink);

        // write manually to the worksheet
        let headers = self.rows.0.remove(0);

        // write intro headers
        let intro_headers = [
            "Date Modified",
            "Number of Files",
            "Series Number",
            "Count Number",
            "File Name",
        ];

        info!("Writing headers");

        for (i, h) in intro_headers.into_iter().enumerate() {
            worksheet
                .write_string(0, i as u16, h)
                .context("error writing header")?;
        }

        for (i, h) in headers.iter().enumerate() {
            if self.rows.1.contains(h) {
                worksheet
                    .write_string_with_format(0, (i + intro_headers.len()) as u16, h, &pink_bg)
                    .context("error writing header")?;
            } else {
                worksheet
                    .write_string(0, (i + intro_headers.len()) as u16, h)
                    .context("error writing header")?;
            }
        }

        info!("Writing cells.");

        for (i, row) in self.rows.0.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                let segment: Vec<(&Format, &str)>;
                if j <= 4 {
                    worksheet
                        .write_string((i + 1) as u32, j as u16, cell)
                        .unwrap();
                    continue;
                }

                let vec = &self
                    .conditions
                    .iter()
                    .filter(|c| cell.contains(&c.data))
                    .map(|c| &c.data)
                    .collect_vec();

                let data = vec.get(0);

                if let Some(d) = &data {
                    let segment_string = Self::split_thing(cell, d);
                    // println!("segment string is: {:?}", &segment_string);
                    segment = segment_string
                        .iter()
                        .map(|s| {
                            if s == **d {
                                (&red, d.as_str())
                            } else {
                                (&default, *s)
                            }
                        })
                        .collect()
                } else {
                    // TODO: redundant allocation? since `split_thing`
                    // already return the original string if there are no
                    // matches
                    segment = vec![(&default, cell.as_str())]
                }

                if cell.trim().is_empty() {
                    worksheet
                        .write_string((i + 1) as u32, j as u16, cell)
                        .unwrap();
                    continue;
                }

                worksheet
                    .write_rich_string((i + 1) as u32, j as u16, segment.as_slice())
                    .unwrap();
            }
        }


        // for (i, row) in self.rows.0.iter().enumerate() {
        //     for (j, cell) in row.iter().enumerate() {
        //         worksheet
        //             .write_string(i as u32, j as u16, cell)
        //             .context("error writing to new cell")?;
        //     }
        // }


        worksheet.autofit();

        info!("saving to a buffer");

        let buf = workbook
            .save_to_buffer()
            .context("failed to save workbook to buffer")
            .unwrap()
            .to_vec();

        info!("sending back response");

        Ok(buf)
    }

    pub fn write_to_vec(&self) -> Vec<Vec<String>> {
        self.rows.0.clone()
    }

    fn split_thing<'a>(haystack: &'a str, needle: &str) -> Vec<&'a str> {
        let Some((l, r)) = haystack.split_once(needle) else {
            return vec![haystack];
        };
        let m = &haystack[l.len()..][..needle.len()];

        if l.is_empty() && r.is_empty() {
            return vec![m];
        }
        if l.is_empty() {
            return vec![m, r];
        }
        if r.is_empty() {
            return vec![l, m];
        }
        vec![l, m, r]
    }
}
