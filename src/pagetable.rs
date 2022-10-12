use crate::prelude::*;

use riscv::register::{self, satp::Mode};

mod regions;

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

pub const BITS_9: u64 = (1 << 10) - 1;

bitflags::bitflags! {
    pub struct Sv48PageTableEntry : u64 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
        const RSW = 0b11 << 8;
        const PPN_0 = BITS_9 << 10;
        const PPN_1 = BITS_9 << 19;
        const PPN_2 = BITS_9 << 28;
        const PPN_3 = BITS_9 << 37;
        const PBMT  = 0b11 << 61;
        const N     = 1 << 63;
    }
}

pub const fn ppn_for_address(addr: usize) -> [u16; 4] {
    assert!(
        addr < (1 << 47),
        "handling of sign extension not implemented"
    );
    let page = addr & !(4096 - 1);
    [
        (page & 0x1FF) as u16,
        ((page >> 9) & 0x1FF) as u16,
        ((page >> 18) & 0x1FF) as u16,
        ((page >> 27) & 0x1FF) as u16,
    ]
}

#[derive(Debug, Clone, Copy)]
#[repr(C, align(4096))]
pub struct Sv48PageTableLeaf {
    entries: [u64; 512],
}

#[derive(Debug, Clone, Copy)]
#[repr(C, align(4096))]
pub struct Sv48PageTableInner<const LEVEL: u8> {
    entries: [u64; 512],
}

impl Sv48PageTableInner<0> {
    fn get_leaf(&self, entry: u32) -> Option<Sv48PageTableLeaf> {
        todo!()
    }
}

impl<const N: u8> Sv48PageTableInner<N>
where
    AssertBool<{ N > 0 }>: IsTrue,
{
    fn get_child(&self, entry: u32) -> Option<Sv48PageTableInner<{ N - 1 }>> {
        todo!()
    }
}

pub struct AssertBool<const B: bool> {}
pub trait IsTrue {}
pub trait IsFalse {}
impl IsTrue for AssertBool<true> {}
impl IsFalse for AssertBool<false> {}
