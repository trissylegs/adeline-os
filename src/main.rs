
#![allow(dead_code)]

#![no_std]
#![no_main]

mod io;
mod pagetable;
mod sbi;
use sbi::*;

use core::fmt::Write;
use fdt_rs::{base::DevTree, prelude::{PropReader, FallibleIterator}};


#[no_mangle]
pub extern "C" fn kmain(heart_id: u32, device_tree: *const u8) -> ! {

    let version = BASE.get_spec_version();

    match version {
        Ok(_value) => {
            /*  */
            let mut writer = sbi::init_io(&BASE).unwrap();
        
            writeln!(writer, "Hello, world").ok();
            writeln!(writer, "heart: {heart_id}").ok();
            writeln!(writer, "device tree: {device_tree:?}").ok();

            let tree = unsafe { DevTree::from_raw_pointer(device_tree) }
                .expect("DevTree::from_raw_pointer");

            print_tree(&mut writer, &tree).ok();

            writeln!(writer).ok();

            loop {
                let ch = writer.get_char();
                if ch.is_none() || ch == Some(b'q') {
                    break;
                }                
            }
            
            let shutdown = BASE.get_extension::<SystemShutdown>().unwrap().unwrap();
            shutdown.shutdown().expect("shudown");
            loop {}
        }
        Err(_error) => {
            /*  */
            loop {
                panic!("{_error:?}")
            }
        }
    }
}

static INDENT_STR: &'static str = "                                ";

fn indent(n: usize) -> &'static str {
    INDENT_STR.split_at(n).0
}

fn print_tree<W>(w: &mut W, tree: &DevTree<'_>) -> core::fmt::Result
    where W: Write+Sized 
{
    let magic = tree.magic();
    let version = tree.version();
    let totalsize = tree.totalsize();

    let boot_cpuid_phys = tree.boot_cpuid_phys();
    let last_comp_version = tree.last_comp_version();
    let off_mem_rsvmap = tree.off_mem_rsvmap();
    let off_dt_struct = tree.off_dt_struct();
    let size_dt_struct = tree.size_dt_struct();
    let off_dt_strings = tree.off_dt_strings();
    let size_dt_strings = tree.off_dt_strings();


    writeln!(w, "DevTree:")?;

    let mut ind = indenter::indented(w);
    ind = ind.with_str(indent(4));
    writeln!(ind, "magic: {magic}")?;
    writeln!(ind, "version: {version}")?;
    writeln!(ind, "totalsize: {totalsize}")?;
    writeln!(ind, "boot_cpuid_phys: {boot_cpuid_phys}")?;
    writeln!(ind, "last_comp_version: {last_comp_version}")?;
    writeln!(ind, "off_mem_rsvmap: {off_mem_rsvmap}")?;
    writeln!(ind, "off_dt_struct: {off_dt_struct}")?;
    writeln!(ind, "size_dt_struct: {size_dt_struct}")?;
    writeln!(ind, "off_dt_strings: {off_dt_strings}")?;
    writeln!(ind, "size_dt_strings: {size_dt_strings}")?;

    writeln!(ind, "reserved_entries:")?;
    ind = ind.with_str(indent(8));
    for re in tree.reserved_entries() {
        
        let address: u64 = re.address.into();
        let size: u64 = re.size.into();
        writeln!(ind, "fdt_reserve_entry: ")?;
        writeln!(ind, "    address: {address:x}")?;
        writeln!(ind, "    size: {size:x}")?;   
    }
    ind = ind.with_str(indent(4));

    writeln!(ind, "nodes:")?;
    ind = ind.with_str(indent(8));

    let mut address_cells = 0;
    let mut size_cells = 0;

    for node in tree.nodes().iterator() {
        if let Ok(node) = node {
            writeln!(ind, "node:")?;
            ind = ind.with_str(indent(12));
            let name = node.name();
            writeln!(ind, "name: {name:?}")?;
            writeln!(ind, "props:")?;
            ind = ind.with_str(indent(16));
            for prop in node.props().iterator() {
                if let Ok(prop) = prop {
                    if let Ok(prop_name) = prop.name() {
                        match prop_name {
                            "reg" if address_cells == 2 && size_cells == 2 => {
                                let address = prop.u64(0).unwrap();
                                let size = prop.u64(1).unwrap();
                                writeln!(ind, "{}: <0x{:x} 0x{:x}>", prop_name, address, size)?;
                            }
                            "reg" if address_cells == 1 && size_cells == 1 => {
                                let address = prop.u32(0).unwrap();
                                let size = prop.u32(1).unwrap();
                                writeln!(ind, "{}: <0x{:x} 0x{:x}>", prop_name, address, size)?;
                            }
                            "reg" if address_cells == 2 || size_cells == 2 => {
                                let value = prop.u64(0).unwrap();
                                writeln!(ind, "{}: <0x{:x}>", prop_name, value)?;
                            }
                            "reg" if address_cells == 1 || size_cells == 1 => {
                                let value = prop.u32(0).unwrap();
                                writeln!(ind, "{}: <0x{:x}>", prop_name, value)?;
                            }
                            "phandle" => {
                                let phandle = prop.phandle(0).unwrap();
                                writeln!(ind, "{prop_name}: <0x{phandle:x}>")?;
                            }
                            "#address-cells" => {
                                let prop_u32 = prop.u32(0).unwrap();
                                address_cells = prop_u32;
                                writeln!(ind, "{prop_name}: <{prop_u32}>")?;
                            }
                            "#size-cells" => {
                                let prop_u32 = prop.u32(0).unwrap();
                                size_cells = prop_u32;
                                writeln!(ind, "{prop_name}: <{prop_u32}>")?;
                            }

                            _ => {
                                if let Ok(prop_str) = prop.str() {
                                    writeln!(ind, "{}: {:?} ({})", prop_name, prop_str, prop_str.len())?; 
                                } else {
                                    writeln!(ind, "{}", prop_name)?;
                                }
                            }
                            
                        }
                    }
                }
            }
        }

        ind = ind.with_str(indent(8));
    }

    Ok(())
}

mod panic {
    use core::panic::PanicInfo;
    use core::fmt::Write;

    use crate::io;
        
    #[panic_handler]
    #[no_mangle]
    pub fn panic(info: &PanicInfo) -> ! {
        if let Some(io) = io() {
            writeln!(io, "{info}").ok();
        }
        abort();
    }

    #[no_mangle]
    pub extern "C" fn abort() -> ! {
        loop {
        }
    }
}
