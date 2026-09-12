mod runtime;

use std::env;
use runtime::engine::execute_apk;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        panic!("usage: apkrunner run <apk>");
    }

    let apk_path = &args[2];
    execute_apk(apk_path);
}