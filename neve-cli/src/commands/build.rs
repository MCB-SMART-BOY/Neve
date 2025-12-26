//! The `neve build` command.

pub fn run(package: Option<&str>) -> Result<(), String> {
    match package {
        Some(pkg) => println!("Building package '{}' (not yet implemented)", pkg),
        None => println!("Building current package (not yet implemented)"),
    }
    Ok(())
}
