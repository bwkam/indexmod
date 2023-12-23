use calamine::Reader;
use calamine::{open_workbook, Xlsx};
use excel_merge::{FileRange, FilesMap, IntoVec, Search};

#[test]
fn test_search() {
    let mut search_1: Xlsx<_> = open_workbook("tests/search_1.xlsx").unwrap();
    let mut search_2: Xlsx<_> = open_workbook("tests/search_2.xlsx").unwrap();

    let rows1: Vec<Vec<String>> =
        FileRange::from(search_1.worksheet_range_at(0).unwrap().unwrap()).into_vec();

    let rows2: Vec<Vec<String>> =
        FileRange::from(search_2.worksheet_range_at(0).unwrap().unwrap()).into_vec();

    let rows = [rows1, rows2].concat();

    let filtered_rows = FilesMap::search(
        &rows,
        &[Search {
            data: "Indigo".to_string(),
            title: None,
            intersections: vec![],
        }],
    )
    .unwrap();

    assert_eq!(filtered_rows.len(), 110);
}
