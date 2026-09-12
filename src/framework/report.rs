use crate::framework::models::Framework;

pub fn print_framework_report(
    framework: &Framework,
) {

    println!("\nFramework");
    println!("====================");

    match framework {

        Framework::Flutter => {
            println!("Detected:\n  Flutter");
        }

        Framework::Unity => {
            println!("Detected:\n  Unity");
        }

        Framework::ReactNative => {
            println!("Detected:\n  React Native");
        }

        Framework::Unreal => {
            println!("Detected:\n  Unreal Engine");
        }

        Framework::Godot => {
            println!("Detected:\n  Godot");
        }

        Framework::Unknown => {
            println!(
                "Detected:\n  Unknown / Native Android"
            );
        }
    }
}