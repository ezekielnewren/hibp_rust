#![feature(test)]


use std::fs;
use std::ops::{Index, IndexMut, Range};

extern crate test;

const DIR_SRC_DATA: &str = "src/data";
const DIR_TESTS_DATA: &str = "tests/data";


#[test]
fn test_test_data_directory() {
    let cd = fs::canonicalize(".");

    let result = fs::metadata(DIR_TESTS_DATA);
    assert!(result.is_ok());

    assert!(result.unwrap().is_dir());
}
