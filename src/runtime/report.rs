use crate::runtime::models::RuntimePlan;

pub fn print_runtime_plan(plan: &RuntimePlan) {

    println!("\n📊 Runtime Plan");
    println!("====================");

    println!("Main Activity: {}", plan.main_activity);

    println!("DEX files: {}", plan.dex_files.len());

    println!("Native libs: {}", plan.native_libs.len());
}