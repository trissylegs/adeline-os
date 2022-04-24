
use std::error::Error;
use cc::Build;
    
fn main() -> Result<(), Box<dyn Error>> {
    Build::new()
        .file("entry.S")
        .flag("-march=rv64gc")
        .flag("-mabi=lp64d")
        .compile("yolo");

    Ok(())
}
