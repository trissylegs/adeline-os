use alloc::vec::Vec;
use anyhow::Error;
use fdt_rs::prelude::*;
use fdt_rs::{base::DevTree, index::DevTreeIndex};

use crate::sbi::base::BASE_EXTENSION;
use crate::sbi::hart::HartId;
use crate::sbi::reset::SystemResetExtension;
use crate::{print, println, sbi};

pub type PhysicalAddress = usize;

pub type PHandle = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddressRange {
    pub base: PhysicalAddress,
    pub len: usize,
}

#[derive(Debug, Clone, derive_builder::Builder)]
#[builder(no_std)]
pub struct HwInfo {
    /// Memory. Currently assuming a single block of RAM.
    pub ram: PhysicalAddressRange,
    // Memory reserved by SBI.
    #[builder(default, setter(each(name = "add_memory_range")))]
    pub reserved_memory: Vec<PhysicalAddressRange>,
    #[builder(setter(each(name = "add_hart")))]
    pub harts: Vec<Hart>,
    pub uart: UartNS16550a,
    pub plic: Plic,
}

#[derive(Debug, Clone, derive_builder::Builder)]
#[builder(no_std)]
pub struct Hart {
    pub phandle: PHandle,
    pub hart_id: HartId,
}

#[derive(Debug, Clone, derive_builder::Builder)]
#[builder(no_std)]
pub struct UartNS16550a {
    pub reg: PhysicalAddressRange,
    pub interrupts: u32,
    pub interrupt_parent: PHandle,
    pub clock_freq: u32,
}

#[derive(Debug, Clone, derive_builder::Builder)]
#[builder(no_std)]
pub struct Plic {
    pub phandle: PHandle,
    pub reg: PhysicalAddressRange,
    pub interrupts_extended: Vec<u8>,
}

pub fn dump_dtb_hex(dtb: *const u8) {
    sbi::init_io();
    let tree = unsafe { DevTree::from_raw_pointer(dtb).map_err(Error::msg).unwrap() };
    let bytes = tree.buf();
    for b in bytes {
        print!("{:02x}", b);
    }
    println!();

    BASE_EXTENSION
        .get_extension::<SystemResetExtension>()
        .unwrap()
        .unwrap()
        .reset(
            crate::sbi::reset::ResetType::Shutdown,
            crate::sbi::reset::ResetReason::NoReason,
        );
    loop {}
}

pub fn dump_dtb(dtb: *const u8) {
    sbi::init_io();

    let tree = unsafe { DevTree::from_raw_pointer(dtb).map_err(Error::msg).unwrap() };
    let index_layout = DevTreeIndex::get_layout(&tree).map_err(Error::msg).unwrap();

    let mut buffer = alloc::vec![0u8; index_layout.size()];
    let slice = buffer.as_mut_slice();

    let index = DevTreeIndex::new(tree, slice).unwrap();

    for node in index.nodes() {
        let name = node.name().unwrap();
        println!("{} {{", name);
        for prop in node.props() {
            let name = prop.name().unwrap();
            let value = prop.raw();
            println!("  {} = {:?}", name, value);
        }
        println!("}}");
    }

    BASE_EXTENSION
        .get_extension::<SystemResetExtension>()
        .unwrap()
        .unwrap()
        .reset(
            crate::sbi::reset::ResetType::Shutdown,
            crate::sbi::reset::ResetReason::NoReason,
        );
    loop {}
}

pub fn walk_dtb(dtb: *const u8) -> anyhow::Result<HwInfo> {
    let tree = unsafe { DevTree::from_raw_pointer(dtb).map_err(Error::msg)? };
    let index_layout = DevTreeIndex::get_layout(&tree).map_err(Error::msg)?;

    let mut buffer = alloc::vec![0u8; index_layout.size()];
    let slice = buffer.as_mut_slice();

    let index = DevTreeIndex::new(tree, slice).map_err(Error::msg)?;

    let mut hwinfo = HwInfoBuilder::default();

    for node in index.compatible_nodes("riscv") {
        let mut hart = HartBuilder::default();

        let mut is_cpu = false;
        for prop in node.props() {
            if prop.name() == Ok("device_type") && prop.str() == Ok("cpu") {
                is_cpu = true;
            }
            if prop.name() == Ok("phandle") {
                if let Ok(value) = prop.phandle(0) {
                    hart.phandle(value);
                }
            }
            if prop.name() == Ok("reg") {
                if let Ok(value) = prop.u32(0) {
                    hart.hart_id(value.into());
                }
            }
        }

        if is_cpu {
            if let Ok(hart) = hart.build() {
                hwinfo.add_hart(hart);
            }
        }
    }

    for node in index.compatible_nodes("ns16550a") {
        let mut uart = UartNS16550aBuilder::default();

        for prop in node.props() {
            match prop.name() {
                Ok("interrupts") => {
                    if let Ok(interrupts) = prop.u32(0) {
                        uart.interrupts(interrupts);
                    }
                }
                Ok("interrupt-parent") => {
                    if let Ok(interrupt_parent) = prop.phandle(0) {
                        uart.interrupt_parent(interrupt_parent);
                    }
                }
                Ok("reg") => {
                    if let (Ok(base), Ok(len)) = (prop.u64(0), prop.u64(1)) {
                        uart.reg(PhysicalAddressRange {
                            base: base as usize,
                            len: len as usize,
                        });
                    }
                }
                Ok("clock-frequency") => {
                    if let Ok(clock_freq) = prop.u32(0) {
                        uart.clock_freq(clock_freq);
                    }
                }
                _ => {}
            }
        }

        if let Ok(uart) = uart.build() {
            hwinfo.uart(uart);
            break;
        }
    }

    for node in index.compatible_nodes("sifive,plic-1.0.0") {
        let mut plic = PlicBuilder::default();
        for prop in node.props() {
            match prop.name() {
                Ok("phandle") => {
                    if let Ok(phandle) = prop.phandle(0) {
                        plic.phandle(phandle);
                    }
                }
                Ok("reg") => {
                    if let (Ok(base), Ok(len)) = (prop.u64(0), prop.u64(1)) {
                        let reg = PhysicalAddressRange {
                            base: base as usize,
                            len: len as usize,
                        };
                        plic.reg(reg);
                    }
                }
                Ok("interrupts-extended") => {
                    let value = prop.raw();
                    plic.interrupts_extended(Vec::from(value));
                }
                _ => {}
            }
        }

        if let Ok(plic) = plic.build() {
            hwinfo.plic(plic);
        }
    }

    for node in index.nodes() {
        if node.name() == Ok("reserved-memory") {
            for range in node.children() {
                let reg = range.props().find(|p| p.name() == Ok("reg"));
            }
        }

        let mut is_ram = false;
        let mut reg = None;
        for prop in node.props() {
            match prop.name() {
                Ok("device_type") => {
                    if prop.str() == Ok("memory") {
                        is_ram = true;
                    }
                }
                Ok("reg") => {
                    if let (Ok(base), Ok(len)) = (prop.u64(0), prop.u64(1)) {
                        reg = Some(PhysicalAddressRange {
                            base: base as usize,
                            len: len as usize,
                        })
                    }
                }
                _ => {}
            }
        }

        if is_ram && reg.is_some() {
            hwinfo.ram(reg.unwrap());
        }
    }

    hwinfo.build().map_err(Error::msg)
}
