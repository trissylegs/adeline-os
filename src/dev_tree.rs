use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Phandle(u32);

#[derive(Debug)]
pub struct MemoryRange {
    base: usize,
    size: usize,
}

pub struct Compatible {
    value: &'static str,
}

impl Compatible {
    const fn new(value: &'static str) -> Self {
        Compatible { value }
    }

    fn list<'a>(&'a self) -> impl Iterator<Item = &'a str> + 'a {
        self.value.split('\0')
    }
}

impl From<&'static str> for Compatible {
    fn from(s: &'static str) -> Self {
        Compatible { value: s }
    }
}

impl fmt::Debug for Compatible {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.list()).finish()
    }
}

#[derive(Debug)]
pub struct Root {
    compatible: Compatible,
    model: &'static str,
    fw_cfg: FwCfg,
    flash: Flash,
    memory: Memory,
    cpus: Cpus,
    soc: Soc,
    chosen: Chosen,
}

#[derive(Debug)]
pub struct FwCfg {
    dma_coherent: bool,
    reg: MemoryRange,
    compatible: Compatible,
}

#[derive(Debug)]
pub struct Flash {
    bank_width: u32,
    reg: &'static [MemoryRange],
    compatible: Compatible,
}

#[derive(Debug)]
pub struct Chosen {
    bootargs: &'static [u8],
    stdout_path: &'static str,
}

#[derive(Debug)]
pub struct Memory {
    device_type: DeviceType,
    reg: MemoryRange,
}

#[derive(Debug)]
pub struct Cpus {
    timebase_frequency: u32,
    cpus: &'static [Cpu],
    cpu_map: CpuMap,
}

#[derive(Debug)]
pub struct Cpu {
    phandle: Phandle,
    device_type: DeviceType,
    reg: u32,
    status: CpuStatus,
    compatible: Compatible,
    riscv_isa: RiscvIsa,
    riscv_mmu: RiscvMmu,
    interrupt_controller: InterruptController,
}

#[derive(Debug)]
pub struct InterruptController {
    interrupt_cells: u32,
    interrupt_controller: bool,
    compatible: Compatible,
    phandle: Phandle,
}

#[derive(Debug)]
pub struct CpuMap {
    clusters: &'static [Cluster],
}

#[derive(Debug)]
pub struct Cluster {
    cores: &'static [CpuCore],
}

#[derive(Debug)]
pub struct CpuCore {
    cpu: Phandle,
}

#[derive(Debug)]
pub struct Soc {
    compatible: Compatible,
    ranges: bool,
    rtc: Rtc,
    uart: Uart,
    pci: Pci,
    plic: Plic,
    clint: Clint,
    address_cells: i32,
    size_cells: i32,
}

#[derive(Debug)]
pub struct Rtc {
    interrupts: u32,
    interrupts_parent: Phandle,
    reg: MemoryRange,
    compatible: Compatible,
}

#[derive(Debug)]
pub struct Uart {
    interrupts: u32,
    interrupts_parent: Phandle,
    clock_frequency: u32,
    reg: MemoryRange,
    compatible: Compatible,
}

#[derive(Debug)]
pub struct Pci {
    reg: MemoryRange,
    dma_coherent: bool,
    device_type: DeviceType,
    compatible: Compatible,
}

#[derive(Debug)]
pub struct Plic {
    phandle: Phandle,
    compatible: Compatible,
    reg: MemoryRange,
    interruptes_extended: InterruptsExtended,
}

#[derive(Debug)]
pub struct Clint {
    interruptes_extended: InterruptsExtended,
    reg: MemoryRange,
    compatible: Compatible,
}

#[derive(Debug)]
pub struct InterruptsExtended {
    value: &'static [u8],
}

#[derive(Debug)]
pub enum CpuStatus {
    Okay,
}

#[derive(Debug)]
pub enum RiscvIsa {
    Unknown,
    Rv64IMAFDCSU,
}

#[derive(Debug)]
pub enum RiscvMmu {
    Unknown,
    Sv39,
    Sv48,
    Sv57,
    Sv64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeviceType {
    Memory,
    Cpu,
    Pci,
}

pub static VIRT: Root = Root {
    compatible: Compatible::new("riscv-virtio"),
    model: "riscv-virtio,qemu",
    fw_cfg: FwCfg {
        dma_coherent: true,
        reg: MemoryRange {
            base: 0x10100000,
            size: 0x18,
        },
        compatible: Compatible::new("qemu,fw-cfg-mmio"),
    },
    flash: Flash {
        bank_width: 0x04,
        reg: &[
            MemoryRange {
                base: 0x20000000,
                size: 0x2000000,
            },
            MemoryRange {
                base: 0x22000000,
                size: 0x2000000,
            },
        ],
        compatible: Compatible::new("cfi-flash"),
    },
    chosen: Chosen {
        bootargs: &[0x00],
        stdout_path: "/soc/uart@10000000",
    },
    memory: Memory {
        device_type: DeviceType::Memory,
        reg: MemoryRange {
            base: 0x80000000,
            size: 0x8000000,
        },
    },
    cpus: Cpus {
        timebase_frequency: 0x989680,
        cpus: &[Cpu {
            phandle: Phandle(0x01),
            device_type: DeviceType::Cpu,
            reg: 0,
            status: CpuStatus::Okay,
            compatible: Compatible::new("riscv"),
            riscv_isa: RiscvIsa::Rv64IMAFDCSU,
            riscv_mmu: RiscvMmu::Sv48,
            interrupt_controller: InterruptController {
                interrupt_cells: 0x01,
                interrupt_controller: true,
                compatible: Compatible::new("riscv,cpu-intc"),
                phandle: Phandle(0x02),
            },
        }],
        cpu_map: CpuMap {
            clusters: &[Cluster {
                cores: &[CpuCore { cpu: Phandle(0x01) }],
            }],
        },
    },
    soc: Soc {
        address_cells: 0x02,
        size_cells: 0x02,
        compatible: Compatible::new("simple-bus"),
        ranges: true,
        rtc: Rtc {
            interrupts: 0x0b,
            interrupts_parent: Phandle(0x03),
            reg: MemoryRange {
                base: 0x101000,
                size: 0x1000,
            },
            compatible: Compatible::new("google,goldfish-rtc"),
        },
        uart: Uart {
            interrupts: 0x0a,
            interrupts_parent: Phandle(0x03),
            clock_frequency: 0x00384000,
            reg: MemoryRange {
                base: 0x10000000,
                size: 0x100,
            },
            compatible: Compatible::new("ns16550a"),
        },
        pci: Pci {
            reg: MemoryRange {
                base: 0x30000000,
                size: 0x10000000,
            },
            dma_coherent: true,
            device_type: DeviceType::Pci,
            compatible: Compatible::new("pci-host-ecam-generic"),
        },
        plic: Plic {
            phandle: Phandle(0x03),
            compatible: Compatible::new("sifive,plic-1.0.0\0riscv,plic0"),
            reg: MemoryRange {
                base: 0xc000000,
                size: 0x210000,
            },
            interruptes_extended: InterruptsExtended {
                value: &[0x02, 0x0b, 0x02, 0x09],
            },
        },
        clint: Clint {
            interruptes_extended: InterruptsExtended {
                value: &[0x02, 0x03, 0x02, 0x07],
            },
            reg: MemoryRange {
                base: 0x2000000,
                size: 0x10000,
            },
            compatible: Compatible::new("sifive,clint0\0riscv,clint0"),
        },
    },
};
