use std::{fs, io};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use crate::{compress_xz, convert_range, download_range, extract_gz, extract_xz, HASH, HashRange, InterpolationSearch};

use futures::stream::{FuturesUnordered};
use futures::StreamExt;
use tokio::runtime::Runtime;
use rayon::prelude::*;
use crate::file_array::{FileArray, FileArrayMut};
use crate::transform::{Transform, TransformConcurrent};

pub struct HIBPDB<'a> {
    pub dbdir: PathBuf,
    pub hash_col: FileArray<'a, HASH>,
    pub frequency_col: FileArray<'a, u64>,
    pub frequency_idx: FileArray<'a, u64>,
    pub password_col: FileArray<'a, u64>,
}

impl<'a> HIBPDB<'a> {
    pub fn open(v: &Path) -> io::Result<Self> {
        let hash_file = v.join("hash.col");
        let frequency_col_file = v.join("frequency.col");
        let frequency_idx_file = v.join("frequency.idx");
        let password_col_file = v.join("password.col");

        Ok(Self {
            dbdir: PathBuf::from(v),
            hash_col: FileArray::open(hash_file.as_path())?,
            frequency_col: FileArray::open(frequency_col_file.as_path())?,
            frequency_idx: FileArray::open(frequency_idx_file.as_path())?,
            password_col: FileArray::open(password_col_file.as_path())?,
        })
    }

    pub fn save(prefix: &Path, hr: HashRange) -> io::Result<()> {
        let file_name = HashRange::name(hr.range);

        let path_tmp = prefix.join(String::from("tmp.")+file_name.as_str());
        let pathname = prefix.join(file_name);
        {
            let mut fd = File::create(&path_tmp)?;
            let out = hr.serialize();
            fd.write_all(out.as_slice())?;
        }
        fs::rename(path_tmp, pathname)?;

        Ok(())
    }

    pub fn load(prefix: &Path, range: u32) -> io::Result<HashRange> {
        let file_name = HashRange::name(range);

        let mut buff: Vec<u8> = Vec::new();
        let mut fd = File::open(prefix.join(file_name))?;
        fd.read_to_end(&mut buff)?;

        return HashRange::deserialize(buff.as_slice());
    }

    pub fn compact(hr: &HashRange) -> io::Result<HashRange> {
        let mut copy = HashRange {
            range: hr.range,
            etag: hr.etag,
            timestamp: hr.timestamp,
            len: 0,
            sum: 0,
            format: hr.format.clone(),
            buff: Vec::new(),
        };

        copy.buff = match hr.format.as_str() {
            "xz" => extract_xz(hr.buff.as_slice())?,
            "gz" => extract_gz(hr.buff.as_slice())?,
            "txt" => hr.buff.clone(),
            _ => return Err(io::Error::new(ErrorKind::InvalidInput, "unsupported file type")),
        };

        copy.buff.retain(|&x| x != b'\r');

        for v in copy.buff.lines() {
            let line = v?;
            let t = u64::from_str_radix(&line[33-5..], 16);
            match t {
                Ok(_) => copy.sum += 1,
                Err(e) => return Err(io::Error::new(ErrorKind::InvalidInput, e.to_string())),
            }

            copy.len += 1;
        }

        copy.buff = compress_xz(copy.buff.as_slice())?;
        copy.format = String::from("xz");

        Ok(copy)
    }

    pub fn update_download_missing<F>(rt: &Runtime, dbdir: &Path, mut f: F) -> io::Result<()> where F: FnMut(u32)  {
        let dir_range = dbdir.join("range/");
        fs::create_dir_all(dir_range.clone()).unwrap();

        let limit0 = 500;
        let limit1 = rayon::current_num_threads()*10;
        let client = reqwest::Client::new();

        let fut = async {
            let mut queue0 = FuturesUnordered::new();
            let mut queue1: Vec<HashRange> = Vec::new();

            let mut range = 0;
            loop {
                if queue0.len() < limit0 && range < (1<<20) {
                    if !dir_range.join(HashRange::name(range)).exists() {
                        queue0.push(download_range(&client, range));
                    }
                    range += 1;
                } else {
                    let result = queue0.next().await.unwrap();
                    match result {
                        Ok(hr) => queue1.push(hr),
                        Err(e) => queue0.push(download_range(&client, e.range)),
                    }
                }

                let downloaded = range >= (1<<20) && queue0.is_empty();
                if downloaded || queue1.len() >= limit1 {
                    queue1.par_iter().for_each(|hr| {
                        let compact = Self::compact(&hr).unwrap();
                        Self::save(dir_range.as_path(), compact).unwrap();
                    });
                    for r in &queue1 {
                        f(r.range);
                    }
                    queue1.clear();
                }

                if downloaded {
                    break;
                }
            }

            Ok::<(), io::Error>(())
        };

        rt.block_on(fut)
    }

    pub fn update_frequency_index(dbdir: &Path) -> io::Result<()> {
        let file_hash = dbdir.join("hash.col");
        let file_freq = dbdir.join("frequency.col");
        let file_freq_index = dbdir.join("frequency.idx");

        let hash_fa = FileArray::<HASH>::open(file_hash.as_path())?;
        let hash_slice = hash_fa.as_slice();

        let frequency_col_fa = FileArray::<u64>::open(file_freq.as_path())?;
        let frequency_col_slice = frequency_col_fa.as_slice();

        let mut frequency_idx_fa: FileArrayMut<u64> = FileArrayMut::open(file_freq_index.as_path(), hash_slice.len())?;
        let frequency_idx_slice = frequency_idx_fa.as_mut_slice();

        for i in 0..frequency_idx_slice.len() {
            frequency_idx_slice[i] = i as u64;
        }

        frequency_idx_slice.par_sort_unstable_by(|i, j| {
            let mut cmp = frequency_col_slice[*j as usize].cmp(&frequency_col_slice[*i as usize]);
            if cmp.is_eq() {
                cmp = hash_slice[*i as usize].cmp(&hash_slice[*j as usize]);
            }
            return cmp;
        });

        frequency_idx_fa.sync()?;

        Ok(())
    }

    pub fn update_construct_columns<F>(dbdir: &Path, mut f: F) -> io::Result<()> where F: FnMut(u32) {
        let dir_range: PathBuf = dbdir.join("range/");

        let mut file_hash = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(dbdir.join("hash.col"))?;

        let mut file_frequency = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(dbdir.join("frequency.col"))?;

        let prefix = dir_range.clone();
        let mut transformer: TransformConcurrent<u32, io::Result<(Vec<u8>, Vec<u8>)>> = TransformConcurrent::new(move |range| {
            let r = convert_range(Self::load(prefix.as_path(), range)?);
            return match r {
                Ok((buff0, freq)) => {
                    let mut buff1: Vec<u8> = Vec::new();
                    for i in freq {
                        let x = i.to_le_bytes();
                        buff1.extend_from_slice(&x);
                    }
                    Ok((buff0, buff1))
                }
                Err(e) => Err(e),
            }
        }, 0);

        let limit = 1000;

        let mut wp = 0u32;
        let mut rp = 0u32;
        while rp < 1<<20 {
            if wp < (1<<20) && wp-rp < limit {
                transformer.add(wp);
                wp += 1;
            } else {
                let (buff, freq) = transformer.take()?;
                file_hash.write_all(buff.as_slice()).unwrap();
                file_frequency.write_all(freq.as_slice()).unwrap();
                f(rp);
                rp += 1;
            }
        }

        let file_password = dbdir.join("password.col");
        let mut password_fa = FileArrayMut::<u64>::open(file_password.as_path(), file_hash.metadata()?.len() as usize)?;
        let mut password_slice = password_fa.as_slice();


        for i in 0..password_slice.len() {
            password_fa.as_mut_slice()[i] = u64::MAX;
        }
        password_fa.sync()?;

        Ok(())
    }

    #[inline]
    pub fn hash(&self) -> &[HASH] {
        self.hash_col.as_slice()
    }

    pub fn find(self: &mut Self, key: &HASH) -> Result<usize, usize> {
        self.hash().interpolation_search(key)
    }

    pub fn len(&self) -> usize {
        self.hash().len()
    }
}






