use std::{fs};

// const DIR_SRC_DATA: &str = "src/data";
const DIR_TESTS_DATA: &str = "tests/data";


use hibp_core::{download_range, HashRange};
use hibp_core::transform::{Transform, TransformSerial};

#[test]
fn test_test_data_directory() {
    let result = fs::metadata(DIR_TESTS_DATA);
    assert!(result.is_ok());

    assert!(result.unwrap().is_dir());
}

#[test]
fn test_transform_serial() {

    let c: fn(u64) -> u64 = |v| v+1;

    let mut t = TransformSerial::new(c);
    let input = 0u64;
    t.add(input);
    let output = t.take();

    assert_eq!(input + 1, output);
}

#[test]
fn test_transform_concurrent() {

    let c: fn(u64) -> u64 = |v| v+1;

    let mut t = TransformSerial::new(c);


    let hi = num_cpus::get() as u64;
    for input in 0u64..hi {
        t.add(input);
    }

    for i in 0u64..hi {
        let output = t.take();
        assert_eq!(i+1, output);
    }
}

#[test]
fn test_download() {

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();


    let client = reqwest::Client::new();

    let result = rt.block_on(download_range(&client, 0));
    assert!(result.is_ok());

    let hr = result.unwrap();

    let buff = hr.serialize();
    let result = HashRange::deserialize(buff.as_slice());
    assert!(result.is_ok());

    let hrr = result.unwrap();

    assert_eq!(hr.range,     hrr.range);
    assert_eq!(hr.etag,      hrr.etag);
    assert_eq!(hr.timestamp, hrr.timestamp);
    assert_eq!(hr.len,       hrr.len);
    assert_eq!(hr.format,    hrr.format);
    assert_eq!(hr.buff,      hrr.buff);
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
        let db = HIBPDB::new(db_directory()).unwrap();

        #[allow(unused_variables)]
        let view = String::from("");

        let percent: usize = (0.23 * (db.len() as f64)) as usize;
        let t = db.index()[percent];
        #[allow(unused_variables)]
        let view = HASH_to_hex(&t);

        match db.index().interpolation_search(&t) {
            Ok(v) => assert_eq!(percent, v),
            Err(_) => assert!(false),
        }

        let percent: usize = (0.90 * (db.len() as f64)) as usize;
        let t = db.index()[percent];
        #[allow(unused_variables)]
        let view = HASH_to_hex(&t);

        match db.index().interpolation_search(&t) {
            Ok(v) => assert_eq!(percent, v),
            Err(_) => assert!(false),
        }
    }


}



