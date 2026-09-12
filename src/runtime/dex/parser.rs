use std::fs;
use crate::runtime::dex::model::DexFile;

pub fn parse_dex(path: &str) -> DexFile {
    let bytes = fs::read(path).expect("cannot read dex");

    // placeholder realista
    // (tu parser aún no existe realmente)
    DexFile::new(vec![])
}