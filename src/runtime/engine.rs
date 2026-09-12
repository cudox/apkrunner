use crate::runtime::dex::parser::parse_dex;
use crate::runtime::executor::Executor;

pub fn execute_apk(apk_path: &str) {
    let dex = parse_dex(apk_path);

    let mut executor = Executor::new();
    executor.execute(dex);
}