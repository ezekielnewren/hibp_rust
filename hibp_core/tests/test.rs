use std::{fs, thread};
use std::ops::{Index, IndexMut, Range};
use tokio::runtime::Builder;

const DIR_SRC_DATA: &str = "src/data";
const DIR_TESTS_DATA: &str = "tests/data";


use std::io::Read;
use hibp_core::{download_range, HashRange};

#[test]
fn test_test_data_directory() {
    let cd = fs::canonicalize(".");

    let result = fs::metadata(DIR_TESTS_DATA);
    assert!(result.is_ok());

    assert!(result.unwrap().is_dir());
}

#[test]
fn test_arbitrary_code_snippet() {

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();


    let client = reqwest::Client::new();

    let result = rt.block_on(download_range(&client, 0));

    assert!(result.is_ok());
}

mod tests {
    use std::env;
    use hibp_core::db::HIBPDB;
    use hibp_core::{HASH_to_hex, InterpolationSearch};

    fn db_directory() -> String {
        env::var("DB_DIRECTORY").unwrap()
    }

    #[test]
    fn test_interpolation_search() {
        let mut db = HIBPDB::new(db_directory()).unwrap();

        let mut view = String::from("");

        let percent: usize = (0.23 * (db.len() as f64)) as usize;
        let t = db.index()[percent];
        view = HASH_to_hex(&t);

        match db.index().interpolation_search(&t) {
            Ok(v) => assert_eq!(percent, v),
            Err(_) => assert!(false),
        }

        let percent: usize = (0.90 * (db.len() as f64)) as usize;
        let t = db.index()[percent];
        view = HASH_to_hex(&t);

        match db.index().interpolation_search(&t) {
            Ok(v) => assert_eq!(percent, v),
            Err(_) => assert!(false),
        }
    }


}



