use crate::println;
use riscv::register::{self, satp::Mode};

pub fn print_current_page_table() {
    let satp = register::satp::read();

    println!("{:?}", satp.mode());
    println!("{:?}", satp.asid());
    println!("{:?}", satp.ppn());
    if satp.mode() == Mode::Bare {
        println!("Base mapping no more details.");
    }
}
