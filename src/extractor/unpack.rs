use std::{fs, path::PathBuf};
use zip::ZipArchive;
use std::fs::File;

pub fn unpack_apk(apk_path: &str, out_dir: &str) -> Vec<PathBuf> {
    let file = File::open(apk_path).expect("APK not found");
    let mut archive = ZipArchive::new(file).expect("Invalid APK");

    fs::create_dir_all(out_dir).unwrap();

    let mut dex_files = Vec::new();

    for i in 0..archive.len() {
        let mut f = archive.by_index(i).unwrap();
        let name = f.name().to_string();

        let out_path = format!("{}/{}", out_dir, name);

        if let Some(parent) = PathBuf::from(&out_path).parent() {
            fs::create_dir_all(parent).unwrap();
        }

        let mut outfile = File::create(&out_path).unwrap();
        std::io::copy(&mut f, &mut outfile).unwrap();

        if name.ends_with(".dex") {
            dex_files.push(PathBuf::from(out_path));
        }
    }

    dex_files
}