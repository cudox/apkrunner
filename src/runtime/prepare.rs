// src/runtime/prepare.rs

use crate::runtime::loader;
use crate::runtime::models::RuntimePlan;

pub fn prepare_runtime(
    app_dir: &str,
    main_activity: &str,
) -> RuntimePlan {

    println!("\nPreparing runtime...");
    println!("====================");

    loader::prepare_runtime(
        app_dir,
        main_activity,
    )
}