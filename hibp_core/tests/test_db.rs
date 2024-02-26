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
    HIBPDB::update_download_missing(get_runtime(), get_dbdir().as_path(), |_| {}).unwrap();
}

#[test]
pub fn test_update_frequency_index() {
    HIBPDB::update_frequency_index(get_dbdir().as_path()).unwrap();
}

