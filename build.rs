
use std::error::Error;
use cc::Build;
    
fn main() -> Result<(), Box<dyn Error>> {
    Build::new()
        .file("entry.S")
        .flag("-mabi=lp64d")
        .compile("yolo");

    Ok(())
}
