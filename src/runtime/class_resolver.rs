use crate::runtime::dex::DexFile;

pub fn resolve_class(
    dex: &DexFile,
    class_name: &str,
) -> bool {

    let normalized =
        class_name.replace(".", "/");

    dex.strings.iter().any(|s| {

        s.contains("Landroid/")
    })
}