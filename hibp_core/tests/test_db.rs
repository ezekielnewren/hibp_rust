use std::env;
use std::path::PathBuf;
use hibp_core::db::HIBPDB;
use hibp_core::get_runtime;

fn get_dbdir() -> PathBuf {
    let t = env::var("DBDIRECTORY").unwrap();
    PathBuf::from(t)
}


#[test]
pub fn test_update_download_missing() {

    let dbdir = get_dbdir();

    HIBPDB::update_download_missing(get_runtime(), dbdir.as_path(), |v| {}).unwrap();
}


