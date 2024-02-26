use std::env;
use std::path::PathBuf;
use hibp_core::db::HIBPDB;
use hibp_core::get_runtime;

fn get_dbdir() -> PathBuf {
    let t = env::var("DBDIRECTORY").unwrap();
    PathBuf::from(t)
}


#[test]
pub fn test_update() {

    let dbdir = get_dbdir();
    let db = HIBPDB::new(dbdir.as_path());

    let status = |v| {};

    let rt = get_runtime();

    HIBPDB::update(rt, dbdir.as_path(), status).unwrap();

}


