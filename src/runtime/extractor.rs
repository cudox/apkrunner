use std::fs::File;
use std::io::{Read};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

pub fn unpack_apk(apk_path: &str, out_dir: &str) -> Vec<PathBuf> {
    let file = File::open(apk_path).expect("cannot open apk");
    let mut archive = ZipArchive::new(file).expect("invalid zip");

    std::fs::create_dir_all(out_dir).unwrap();

    let mut extracted = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().to_string();

        let outpath = Path::new(out_dir).join(&name);

        if let Some(parent) = outpath.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }

        let mut outfile = File::create(&outpath).unwrap();
        std::io::copy(&mut file, &mut outfile).unwrap();

        extracted.push(outpath);
    }

    extracted
}