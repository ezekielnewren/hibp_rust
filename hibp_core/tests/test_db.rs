use std::env;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use hibp_core::db::HIBPDB;
use hibp_core::{compute_offset, get_runtime, HASH_to_hex};
use hibp_core::minbitrep::MinBitRep;

fn get_dbdir() -> PathBuf {
    let t = env::var("DBDIRECTORY").unwrap();
    PathBuf::from(t)
}


#[test]
pub fn test_update_download_missing() {
    HIBPDB::update_download_missing(get_runtime(), get_dbdir().as_path(), |_| {}).unwrap();
}

#[test]
pub fn test_update_construct_columns() {
    HIBPDB::update_construct_columns(get_dbdir().as_path(), |_|{}).unwrap();
}

#[test]
pub fn test_update_frequency_index() {
    HIBPDB::update_frequency_index(get_dbdir().as_path()).unwrap();
}

#[test]
pub fn test_export_db() {

    let db = HIBPDB::open(get_dbdir().as_path()).unwrap();

    let freq_idx = db.frequency_idx.as_slice();
    let hash_col = db.hash_col.as_slice();
    let freq_col = db.frequency_col.as_slice();

    let file_dump = PathBuf::from("/tmp/dump.txt");
    let mut fd = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(file_dump).unwrap();

    let mut card = [0u64; 64];

    for i in freq_idx {
        let freq = freq_col[*i as usize];
        let x = MinBitRep::minbit(freq);
        card[x as usize] += 1;
    }

    for i in 0..card.len() {
        let line = format!("{}:{}\n", i, card[i]);
        fd.write_all(line.as_bytes()).unwrap();
    }

}


#[test]
pub fn test_chunked_range() {

    let db = HIBPDB::open(get_dbdir().as_path()).unwrap();

    let hash_col = db.hash_col.as_slice();

    let file_dump = PathBuf::from("/tmp/dump.txt");
    let mut fd = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(file_dump).unwrap();

    let mut card = [0u64; 64];

    for i in 0..card.len() {
        let line = format!("{}:{}\n", i, card[i]);
        fd.write_all(line.as_bytes()).unwrap();
    }
}


#[test]
pub fn test_compute_offset() {
    let db = HIBPDB::open(get_dbdir().as_path()).unwrap();

    let hash_col = db.hash_col.as_slice();

    let bit_len = 29;
    let off = compute_offset(hash_col, bit_len);

    assert!(off.len() > 0);

    let mut empty = 0;
    for i in 0..(1<<bit_len) {
        let diff = off[i+1]-off[i];
        if diff == 0 {
            empty += 1;
        }
    }

    assert!(empty >= 0);
}


