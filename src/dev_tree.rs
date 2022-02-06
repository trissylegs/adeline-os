
use core::{fmt, mem};

use alloc::{string::String, vec::Vec, boxed::Box, borrow::ToOwned};
use fdt_rs::{base::{DevTree, iters::DevTreeIter, DevTreeItem}, error::DevTreeError, prelude::PropReader};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Phandle(u32);

#[derive(Debug)]
pub struct MemoryRange {
    base: usize,
    size: usize,
}

pub struct Compatible {
    value: String,
}

impl Compatible {
    fn list<'a>(&'a self) -> impl Iterator<Item=&'a str> + 'a {
        self.value.split('\0')
    }
}

impl<'a> From<&'a str> for Compatible {
    fn from(s: &'a str) -> Self {
        Compatible {
            value: s.into()
        }
    }
}

impl fmt::Debug for Compatible {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.list())
            .finish()
    }
}

#[derive(Debug)]
pub struct Root {    
    compatible: Compatible,
    model: String,
    fw_cfg: FwCfg,
    flash: Flash,
    memory: Memory,
    cpus: Cpus,
    soc: Soc,
}

#[derive(Debug)]
pub struct FwCfg {
    dma_coherent: bool,
    reg: MemoryRange,  
    compatible: String,
}

#[derive(Debug)]
pub struct Flash {
    bank_width: u32,
    reg: Vec<MemoryRange>,    
    compatible: String,
}

#[derive(Debug)]
pub struct Chosen {
    bootargs: Vec<u8>,
    stdout_path: String,
}

#[derive(Debug)]
pub struct Memory {
    device_type: DeviceType,
    reg: MemoryRange,
}

#[derive(Debug)]
pub struct Cpus {
    timebase_frequency: u32,
    cpus: Vec<Cpu>,
    cpu_map: CpuMap,
}

#[derive(Debug)]
pub struct Cpu {
    phandle: Phandle,
    device_type: DeviceType,
    reg: u32,
    status: CpuStatus,
    compatible: String,
    riscv_isa: Option<RiscvIsa>,
    riscv_mmu: Option<RiscvMmu>,
    interrupt_controller: InterruptController,
}

#[derive(Debug)]
pub struct InterruptController {
    compatible: String,
    phandle: Phandle,
}

#[derive(Debug)]
pub struct CpuMap {
    clusters: Vec<Cluster>, 
}

#[derive(Debug)]
pub struct Cluster {
    cores: Vec<CpuCore>,
}

#[derive(Debug)]
pub struct CpuCore {
    cpu: Phandle,
}

#[derive(Debug)]
pub struct Soc {
    compatible: String,
    ranges: bool,
    rtc: Rtc,
    uart: Uart,
    pci: Pci,
    plic: Plic,
    clint: Clint,
}

#[derive(Debug)]
pub struct Rtc {
    interrupts: u32,
    interrupts_parent: Phandle,
    reg: MemoryRange,
    compatible: String,
}

#[derive(Debug)]
pub struct Uart {
    interrupts: u32,
    interrupts_parent: Phandle,
    clock_frequency: u32,
    reg: MemoryRange,
    compatible: String,
}

#[derive(Debug)]
pub struct Pci {
    reg: MemoryRange,
    // TODO
    dma_coherent: bool,
    device_type: DeviceType,
    compatible: String,    
}

#[derive(Debug)]
pub struct Plic {
    phandle: Phandle,
    compatible: String,
    reg: MemoryRange,
    interruptes_extended: InterruptsExtended,
}

#[derive(Debug)]
pub struct Clint {
    interruptes_extended: InterruptsExtended,
    reg: MemoryRange,
    compatible: String,
}

#[derive(Debug)]
pub struct InterruptsExtended {
    value: Vec<u8>,
}

#[derive(Debug)]
pub enum CpuStatus {
    Okay,
}

#[derive(Debug)]
pub enum RiscvIsa {
    Rv64IMAFDCSU,
}

#[derive(Debug)]
pub enum RiscvMmu {
    Sv48,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeviceType {
    Memory,
    Cpu,
    Pci,
}

pub fn parse(t: DevTree<'_>) -> Result<Box<Root>, DevTreeError> {
    let mut items = t.items();
    Ok(Box::new(Root::parse(&mut items)?))
}

pub struct InheritedNumbers {
    address_cells: u32,
    size_cells: u32,
}

impl Root {
    fn parse(items: &mut DevTreeIter) -> Result<Root, DevTreeError> {
        let mut compatible: Option<Compatible> = None;
        let mut model: Option<String> = None;
        let mut fw_cfg: Option<FwCfg> = None;
        let mut flash: Option<Flash> = None;
        let mut memory: Option<Memory> = None;
        let mut cpus: Option<Cpus> = None;
        let mut soc: Option<Soc> = None;

        let mut ihn = InheritedNumbers {
            address_cells: 0,
            size_cells: 0,
        };

        loop {
            match items.next_item()? {
                Some(DevTreeItem::Prop(p)) => {
                    match p.name()? {
                        "compatible" => compatible = Some(p.str()?.into()),
                        "model" => model = Some(p.str()?.into()),
                        "#address-cells" => ihn.address_cells = p.u32(0)?,
                        "#size-cells" => ihn.size_cells = p.u32(0)?,
                        _ => (),
                    }
                },
                Some(DevTreeItem::Node(n)) => {
                    let name = n.name()?;
                    let node_type = if let Some((ty, _addr)) = name.split_once('@') {
                        ty
                    } else {
                        name
                    };
                    todo!()
                }
                None => return Ok(Root {
                    compatible: compatible.unwrap(),
                    model: model.unwrap(),
                    fw_cfg: fw_cfg.unwrap(),
                    flash: flash.unwrap(),
                    memory: memory.unwrap(),
                    cpus: cpus.unwrap(),
                    soc: soc.unwrap(),
                })
            }
        }
    }
}