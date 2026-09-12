use std::path::Path;

pub fn extraction_exists(
    package: &str,
) -> bool {

    let home =
        std::env::var("HOME")
        .unwrap();

    let path =
        format!(
            "{}/.apkrunner/apps/{}",
            home,
            package
        );

    Path::new(&path).exists()
}