use calamine::Reader;
use calamine::{open_workbook, Xlsx};
use excel_merge::search::Search;
use excel_merge::FilesMap;
use itertools::Itertools;

// TODO: Generate test files programmatically and handle all cases, also add #[should_panic] tests
#[test]
fn search() {
    let mut search_1: Xlsx<_> = open_workbook("tests/search_3.xlsx").unwrap();
    let mut search_2: Xlsx<_> = open_workbook("tests/search_4.xlsx").unwrap();

    let s1 = search_1.worksheet_range_at(0).unwrap().unwrap();
    let rows1 = s1.rows().collect_vec();

    let s2 = search_2.worksheet_range_at(0).unwrap().unwrap();
    let rows2 = s2.rows().collect_vec();

    let rows = [rows2, rows1].concat();

    let _filtered_rows = FilesMap::search(
        rows,
        &[
            Search {
                data: "ZZNC1900002^001350NI".to_string(),
                title: None,
                intersections: vec![],
            },
            Search {
                data: "QGA0000000500A0000NI".to_string(),
                title: None,
                intersections: vec![Search {
                    data: "N059".to_string(),
                    title: Some("海關編號 Mã hải quan".to_string()),
                    intersections: vec![],
                }],
            },
        ],
    )
    .unwrap()
    .write_to_vec();

    // assert_eq!(filtered_rows.len(), 111);
}
