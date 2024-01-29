use core::{
    fmt::{Debug, Formatter},
    mem::size_of,
    ops::{Range},
    str,
};

use alloc::{vec::Vec};
use anyhow::Error;
use fdt_rs::{base::DevTree, index::DevTreeIndex, prelude::*, spec::Phandle, error::DevTreeError};
use spin::Once;

use crate::{
    basic_allocator,
    isr::plic::InterruptId,
    linker_info::{bss, data, rodata, text},
    prelude::*,
    sbi::{
        hart::HartId,
        reset::{shutdown, system_reset_extension},
    }, pagetable::{BigPage, PAGE_SIZE},
};

static HW_INFO: Once<HwInfo> = Once::INIT;

pub type PHandle = u32;

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct PhysicalAddressRange {
    pub kind: PhysicalAddressKind,
    pub description: &'static str,
    pub start: u64,
    pub end: u64,
}

impl PhysicalAddressRange {
    pub fn new(range: Range<u64>, kind: PhysicalAddressKind, description: &'static str) -> Self {
        PhysicalAddressRange {
            kind,
            start: range.start,
            end: range.end,
            description,
        }
    }

    pub fn page_numbers(&self) -> impl Iterator<Item = u64> {
        let mut current = self.start;
        let end = self.end;
        core::iter::from_fn(move || {
            let next = current;
            current += 4096;
            if next < end {
                Some(next / 4096)
            } else {
                None
            }
        })
    }

    pub fn big_pages(&self) -> impl Iterator<Item = BigPage> {
        let mut current = self.start;
        let end = page_end(self.end);
        core::iter::from_fn(move || {
            let next = current;
            if next >= end {
                return None;
            }

            let remaining = end - next;

            let page = BigPage::page_for(next, remaining);
            current += page.size();
            return Some(page);
        })
    }

    pub fn as_range(&self) -> Range<u64> {
        self.start..self.end
    }
}

impl Debug for PhysicalAddressRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PhysicalAddressRange")
            .field("kind", &self.kind)
            .field("description", &self.description)
            .field(
                "range",
                &format_args!("0x{:08x}..0x{:08x}", self.start, self.end),
            )
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PhysicalAddressKind {
    /// Address contains nothing
    Usable,
    /// Reserved by SBI.
    Reserved,
    /// Memory mapped io
    Mmio,
    /// Executable RAM
    Executable,
    /// Read-only RAM
    ReadOnly,
    /// Read-write RAM
    Writable,
}

#[derive(Debug, Clone, derive_builder::Builder)]
#[builder(no_std)]
pub struct HwInfo {
    pub timebase_freq: u64,

    /// Memory. Currently assuming a single block of RAM.
    #[builder(default, setter(each(name = "add_memory")))]
    pub ram: Vec<PhysicalAddressRange>,
    // Memory reserved by SBI.
    #[builder(default, setter(each(name = "add_reserved_memory")))]
    pub reserved_memory: Vec<PhysicalAddressRange>,
    #[builder(setter(each(name = "add_hart")))]
    pub harts: Vec<Hart>,
    pub uart: UartNS16550a,
    pub plic: Plic,
    pub clint: Clint,

    pub rtc: Rtc,
}

#[derive(Debug, Clone, derive_builder::Builder)]
#[builder(no_std)]
pub struct Hart {
    pub name: String,
    pub phandle: PHandle,
    pub hart_id: HartId,
    pub interrupt_handle: PHandle,
}

#[derive(Debug, Clone, derive_builder::Builder)]
#[builder(no_std)]
pub struct UartNS16550a {
    pub name: String,
    pub reg: PhysicalAddressRange,
    pub interrupt: InterruptId,
    pub interrupt_parent: PHandle,
    pub clock_freq: u32,
}

#[derive(Debug, Clone, derive_builder::Builder)]
#[builder(no_std)]
pub struct Plic {
    pub name: String,
    pub phandle: PHandle,
    pub number_of_sources: u32,
    pub reg: PhysicalAddressRange,
    // Specified by interrupts extended.
    pub contexts: Vec<InterruptContext>,
}

#[derive(Debug, Clone, derive_builder::Builder)]
#[builder(no_std)]
pub struct Clint {
    pub name: String,
    pub reg: PhysicalAddressRange,
    pub contexts: Vec<InterruptContext>,
}

#[derive(Debug, Clone, Copy)]
pub struct InterruptContext {
    pub index: usize,
    pub interrupt_phandle: Phandle,
    // I can't figure out what this is.
    // If it's u32::MAX it's not present.
    // If it's '9' it is.
    pub interrupt_cause: InterruptCause,
    pub hart_id: HartId,
}

#[derive(Debug, Clone, derive_builder::Builder)]
#[builder(no_std)]
pub struct Rtc {
    pub name: String,
    pub interrupt: InterruptId,
    pub interrupt_parent: Phandle,
    pub reg: PhysicalAddressRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InterruptCause {
    /// Supervisor software interrupt
    SupervisorSoft = 1,
    /// Virtual supervisor software interrupt
    VirtualSupervisorSoft = 2,
    /// Machine software interrupt
    MachineSoft = 3,
    /// Supervisor timer interrupt
    SupervisorTimer = 5,
    /// Virtual supervisor timer interrupt
    VirtualSupervisorTimer = 6,
    /// Machine timer interrupt
    MachineTimer = 7,
    /// Supervisor external interrupt
    SupervisorExternal = 9,
    /// Virtual supervisor external interrupt
    VirtualSupervisorExternal = 10,
    /// Machine external interrupt
    MachineExternal = 11,
    /// Supervisor guest external interrupt
    SupervisorGuestExternal = 12,
}

impl TryFrom<u32> for InterruptCause {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        use InterruptCause::*;
        match value {
            1 => Ok(SupervisorSoft),
            2 => Ok(VirtualSupervisorSoft),
            3 => Ok(MachineSoft),
            5 => Ok(SupervisorTimer),
            6 => Ok(VirtualSupervisorTimer),
            7 => Ok(MachineTimer),
            9 => Ok(SupervisorExternal),
            10 => Ok(VirtualSupervisorExternal),
            11 => Ok(MachineExternal),
            12 => Ok(SupervisorGuestExternal),
            _ => Err(anyhow::anyhow!("Invalid interrupt cause: {}", value)),
        }
    }
}

impl Into<u32> for InterruptCause {
    fn into(self) -> u32 {
        self as u32
    }
}

pub fn dump_dtb_hex(dtb: *const u8) {
    // sbi::init_io().ok();
    let tree = unsafe { DevTree::from_raw_pointer(dtb).map_err(Error::msg).unwrap() };
    let bytes = tree.buf();
    for b in bytes {
        print!("{:02x}", b);
    }
    println!();

    system_reset_extension()
        .reset(
            crate::sbi::reset::ResetType::Shutdown,
            crate::sbi::reset::ResetReason::NoReason,
        )
        .unwrap();
}

pub fn dump_dtb(dtb: *const u8) {
    // sbi::init_io().unwrap();

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

    shutdown();
}

const fn page_end(addr: u64) -> u64 {
    let rem = addr % PAGE_SIZE;
    if rem == 0 {
        addr
    } else {
        addr + (PAGE_SIZE - rem)
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct DtbRef(*const u8);

impl DtbRef {
    unsafe fn new(ptr: *const u8) -> Self {
        DtbRef(ptr)
    }

    fn dev_tree<'a>(&'a self) -> Result<DevTree<'a>, DevTreeError> {
        unsafe { DevTree::from_raw_pointer(self.0) }
    }

    pub fn start(&self) -> u64 {
        self.0 as u64
    }
}

pub fn setup_dtb(dtb: DtbRef) -> &'static HwInfo {
    HW_INFO.call_once(|| {
        let dt = match dtb.dev_tree() {
            Ok(dt) => dt,
            Err(err) => {
                panic!("Error parsing Device Tree: {}", err);
            }
        };

        let hwinfo = match walk_dtb(dt) {
            Ok(hwinfo) => hwinfo,
            Err(err) => {
                panic!("Error parsing Device Tree: {}", err);
            }
        };

        hwinfo
    })
}

fn walk_dtb<'a>(tree: DevTree<'a>) -> anyhow::Result<HwInfo> {
    let index_layout = DevTreeIndex::get_layout(&tree).map_err(Error::msg)?;

    let mut index_buffer = alloc::vec![0u8; index_layout.size()];
    let slice = index_buffer.as_mut_slice();

    let index = DevTreeIndex::new(tree, slice).map_err(Error::msg)?;

    let mut hwinfo = HwInfoBuilder::default();

    for node in index.compatible_nodes("riscv") {
        let mut hart = HartBuilder::default();
        let mut is_cpu = false;

        if let Ok(name) = node.name() {
            hart.name(name.into());
        } else {
            continue;
        };

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

        for child in node.children() {
            let mut phandle = None;
            let mut compatible = false;
            for prop in child.props() {
                match prop.name() {
                    Ok("compatible") => {
                        if prop.str().unwrap().contains("riscv,cpu-intc") {
                            compatible = true;
                        }
                    }
                    Ok("phandle") => {
                        phandle = prop.phandle(0).ok();
                    }
                    _ => {}
                }
            }

            if compatible && phandle.is_some() {
                hart.interrupt_handle(phandle.unwrap());
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

        if let Ok(name) = node.name() {
            uart.name(name.into());
        } else {
            continue;
        };

        for prop in node.props() {
            match prop.name() {
                Ok("interrupts") => {
                    if let Ok(interrupts) = prop.u32(0) {
                        uart.interrupt(InterruptId::from(interrupts));
                    }
                }
                Ok("interrupt-parent") => {
                    if let Ok(interrupt_parent) = prop.phandle(0) {
                        uart.interrupt_parent(interrupt_parent);
                    }
                }
                Ok("reg") => {
                    if let (Ok(base), Ok(len)) = (prop.u64(0), prop.u64(1)) {
                        uart.reg(PhysicalAddressRange::new(
                            base..base + len,
                            PhysicalAddressKind::Mmio,
                            "uart",
                        ));
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
        if let Ok(name) = node.name() {
            plic.name(name.into());
        } else {
            continue;
        };

        for prop in node.props() {
            match prop.name() {
                Ok("phandle") => {
                    if let Ok(phandle) = prop.phandle(0) {
                        plic.phandle(phandle);
                    }
                }
                Ok("riscv,ndev") => {
                    plic.number_of_sources(prop.u32(0).unwrap());
                }
                Ok("reg") => {
                    if let (Ok(base), Ok(len)) = (prop.u64(0), prop.u64(1)) {
                        let reg = PhysicalAddressRange::new(
                            base..(base + len),
                            PhysicalAddressKind::Mmio,
                            "plic",
                        );
                        plic.reg(reg);
                    }
                }
                Ok("interrupts-extended") => {
                    plic.contexts(parse_interrupt_extended(prop, &hwinfo));
                }

                _ => {}
            }
        }

        if let Ok(plic) = plic.build() {
            hwinfo.plic(plic);
        }
    }

    for node in index.compatible_nodes("sifive,clint0") {
        let mut clint = ClintBuilder::default();
        let name = node.name().expect("clint node does not have name");
        clint.name(name.into());

        for prop in node.props() {
            match prop.name().expect("clint node failed get prop name") {
                "reg" => {
                    // OpenSBI protects clint0.
                    let kind = PhysicalAddressKind::Reserved;
                    let base = prop
                        .u64(0)
                        .unwrap_or_else(|err| panic!("failed to read {name}/reg[0] as u64: {err}"));
                    let len = prop
                        .u64(1)
                        .unwrap_or_else(|err| panic!("failed to read {name}/reg[1] as u64: {err}"));
                    clint.reg(PhysicalAddressRange::new(base..(base + len), kind, "clint"));
                }
                "interrupts-extended" => {
                    clint.contexts(parse_interrupt_extended(prop, &hwinfo));
                }

                _ => {}
            }
        }
        hwinfo.clint(clint.build().expect("failed to build clint"));
    }

    for node in index.compatible_nodes("google,goldfish-rtc") {
        let mut rtc = RtcBuilder::default();

        rtc.name(node.name().expect("rtc: node has no name").into());

        for prop in node.props() {
            match prop.name().expect("rtc: prop has no name") {
                "interrupts" => {
                    let int = InterruptId::new(prop.u32(0).expect("interrupts has no data"))
                        .expect("rtc: interrupt numbers cannot be zero");
                    rtc.interrupt(int);
                }
                "interrupt-parent" => {
                    let val = prop
                        .phandle(0)
                        .expect("rtc: interrupt-parent requires parent");

                    rtc.interrupt_parent(val);
                }
                "reg" => {
                    let reg_base = prop.u64(0).expect("rtc: error getting reg[0]");
                    let reg_len = prop.u64(1).expect("rtc: error getting reg[1]");
                    rtc.reg(PhysicalAddressRange::new(
                        reg_base..(reg_base + reg_len),
                        PhysicalAddressKind::Mmio,
                        "rtc",
                    ));
                }
                _ => {}
            }
        }
        hwinfo.rtc(rtc.build().unwrap());
    }

    for node in index.nodes() {
        if node.name() == Ok("reserved-memory") {
            for range in node.children() {
                if let Some(reg) = range.props().find(|p| p.name() == Ok("reg")) {
                    let base = reg.u64(0).unwrap();
                    let len = reg.u64(1).unwrap();
                    hwinfo.add_reserved_memory(PhysicalAddressRange::new(
                        base..(base + len),
                        PhysicalAddressKind::Reserved,
                        "reserved-memory",
                    ));
                    // Only prop we need or expect to find.
                    break;
                }
            }
            // We're done with this node.
            continue;
        }

        let mut is_ram = false;
        let mut reg = None;
        for prop in node.props() {
            // let name = node.name().unwrap();
            match prop.name() {
                Ok("device_type") => {
                    if prop.str() == Ok("memory") {
                        is_ram = true;
                    }
                }
                Ok("reg") => {
                    if let (Ok(base), Ok(len)) = (prop.u64(0), prop.u64(1)) {
                        reg = Some(PhysicalAddressRange::new(
                            base..(base + len),
                            PhysicalAddressKind::Usable,
                            "",
                        ));
                    }
                }
                Ok("timebase-frequency") => {
                    match prop.length() {
                        4 => hwinfo.timebase_freq(prop.u32(0).unwrap() as u64),
                        8 => hwinfo.timebase_freq(prop.u64(0).unwrap()),
                        _ => panic!("Unexpected timebase-frequency value: {:?}", prop.raw()),
                    };
                }
                _ => {}
            }
        }

        if is_ram && reg.is_some() {
            let mut reg = reg.unwrap();
            reg.description = "RAM";
            hwinfo.add_memory(reg);
        }
    }

    hwinfo.build().map_err(Error::msg)
}

fn parse_interrupt_extended<'a>(
    prop: fdt_rs::index::DevTreeIndexProp,
    hwinfo: &'a HwInfoBuilder,
) -> Vec<InterruptContext> {
    let entries = prop.length() / size_of::<Phandle>() / 2;
    let mut result = Vec::new();

    for index in 0..entries {
        let phandle_offset = 2 * index as usize;
        let interrupt_cause_offset = phandle_offset + 1;

        let phandle = prop
            .phandle(phandle_offset)
            .expect("failed to read phandle");

        if let Ok(cause) = InterruptCause::try_from(prop.u32(interrupt_cause_offset).unwrap()) {
            if let Some(hart) = hwinfo
                .harts
                .as_ref()
                .unwrap()
                .iter()
                .find(|h| h.interrupt_handle == phandle)
            {
                result.push(InterruptContext {
                    index: phandle_offset,
                    interrupt_phandle: phandle,
                    interrupt_cause: cause,
                    hart_id: hart.hart_id,
                });
            }
        }
    }
    result
}

pub struct MemoryLayout {
    pub executable_memory: PhysicalAddressRange,
    pub read_only_memory: PhysicalAddressRange,
    pub mutable_memory: PhysicalAddressRange,
    pub mmio: Vec<PhysicalAddressRange>,
    pub reserved_memory: Vec<PhysicalAddressRange>,
    pub tree_memory: PhysicalAddressRange,
    pub unused_memory: Vec<PhysicalAddressRange>,
}

impl HwInfo {
    pub fn memory_layout(&self) -> Vec<PhysicalAddressRange> {
        let mut layout = vec![];
        layout.push(PhysicalAddressRange::new(
            text(),
            PhysicalAddressKind::Executable,
            ".text",
        ));
        layout.push(PhysicalAddressRange::new(
            rodata(),
            PhysicalAddressKind::ReadOnly,
            ".rodata",
        ));
        layout.push(PhysicalAddressRange::new(
            data(),
            PhysicalAddressKind::Writable,
            ".data",
        ));
        layout.push(PhysicalAddressRange::new(
            bss(),
            PhysicalAddressKind::Writable,
            ".bss",
        ));
        layout.push(self.uart.reg.clone());
        layout.push(self.plic.reg.clone());
        layout.push(self.rtc.reg.clone());
        for rm in self.reserved_memory.iter() {
            layout.push(rm.clone());
        }

        layout.push(basic_allocator::heap_range());
        // layout.push(self.tree_range);
        /*
        let spare_start = if self.tree_range.end % 4096 == 0 {
            self.tree_range.end
        } else {
            self.tree_range.end.next_multiple_of(4096)
        };

        layout.push(PhysicalAddressRange::new(
            spare_start..(self.ram[0].end),
            PhysicalAddressKind::Writable,
            "spare",
        ));
*/
        layout.sort_by_key(|range| range.start);
        for r in layout.windows(2) {
            assert!(r[0].end <= r[1].start, "{} does not finish before {}", r[0].description, r[1].description);
        }
        layout
    }
}
