use crate::prelude::*;
use riscv::register::{self, satp::Mode};

pub fn print_current_page_table() {
    let satp = register::satp::read();
    
    println!("PageTable: {{");
    println!("  mode: {:?}", satp.mode());
    println!("  asid: {:?}", satp.asid());
    println!("  ppn:  {:?}", satp.ppn());
    println!("}}");
    if satp.mode() == Mode::Bare {
        println!("Base mapping no more details.");
    }
}
