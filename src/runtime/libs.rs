use std::fs;

pub fn find_native_libs(
    app_dir: &str,
) -> Vec<String> {

    let mut libs =
        Vec::new();

    let lib_dir =
        format!("{}/lib", app_dir);

    if let Ok(arches) =
        fs::read_dir(lib_dir)
    {
        for arch in arches {

            let arch =
                arch.unwrap();

            let arch_path =
                arch.path();

            if let Ok(entries) =
                fs::read_dir(arch_path)
            {
                for file in entries {

                    let file =
                        file.unwrap();

                    let path =
                        file.path();

                    if let Some(name) =
                        path.file_name()
                    {
                        let name =
                            name
                            .to_string_lossy();

                        if name.ends_with(".so")
                        {
                            libs.push(
                                path
                                .display()
                                .to_string()
                            );
                        }
                    }
                }
            }
        }
    }

    libs
}