///
/// Copied from `uart_16550` crate
///
/// Copyright (c) 2019 Lachlan Sneff
/// Copyright (c) 2019 Philipp Oppermann
/// Copyright (c) 2022 Triss Healy
///
use core::{
    fmt, str,
    sync::atomic::{AtomicPtr, Ordering},
};

use crate::{
    isr::plic::{self, InterruptId},
    wait_for,
};

bitflags::bitflags! {
    /// Line status flags
    struct LineStsFlags: u8 {
        const INPUT_FULL = 1;
        // 1 to 4 unknown
        const OUTPUT_EMPTY = 1 << 5;
        // 6 and 7 unknown
    }
}

#[derive(Debug)]
/// A memory-mapped UART.
pub struct MmioSerialPort {
    int_id: InterruptId,
    data: AtomicPtr<u8>,
    int_en: AtomicPtr<InterruptEnable>,
    fifo_ctrl: AtomicPtr<u8>,
    line_ctrl: AtomicPtr<u8>,
    modem_ctrl: AtomicPtr<ModemControlRegister>,
    line_sts: AtomicPtr<u8>,
}

bitflags::bitflags! {
    pub struct ModemControlRegister : u8 {
        const DATA_TERMINAL_READY = 1 << 0;
        const REQUEST_TO_SEND = 1 << 1;
        const OUT_1 = 1 << 2;
        const OUT_2 = 1 << 3;
        const LOOP = 1 << 4;
    }
}

bitflags::bitflags! {
    pub struct InterruptEnable : u8 {
        const RDI = 0x01;
        const THRI = 0x02;
        const RLSI = 0x04;
        const MSI = 0x08;
    }
}

impl MmioSerialPort {
    /// Creates a new UART interface on the given memory mapped address.
    ///
    /// This function is unsafe because the caller must ensure that the given base address
    /// really points to a serial port device.

    pub unsafe fn new(base: usize, int_id: InterruptId) -> Self {
        let base_pointer = base as *mut u8;
        Self {
            int_id,
            data: AtomicPtr::new(base_pointer),
            int_en: AtomicPtr::new(base_pointer.add(1) as *mut InterruptEnable),
            fifo_ctrl: AtomicPtr::new(base_pointer.add(2)),
            line_ctrl: AtomicPtr::new(base_pointer.add(3)),
            modem_ctrl: AtomicPtr::new(base_pointer.add(4) as *mut ModemControlRegister),
            line_sts: AtomicPtr::new(base_pointer.add(5)),
        }
    }

    /// Initializes the memory-mapped UART.
    ///
    /// The default configuration of [38400/8-N-1](https://en.wikipedia.org/wiki/8-N-1) is used.
    pub fn init(&mut self) -> anyhow::Result<()> {
        let self_int_en = self.int_en.load(Ordering::Relaxed);
        let self_line_ctrl = self.line_ctrl.load(Ordering::Relaxed);
        let self_data = self.data.load(Ordering::Relaxed);
        let self_fifo_ctrl = self.fifo_ctrl.load(Ordering::Relaxed);
        let self_modem_ctrl = self.modem_ctrl.load(Ordering::Relaxed);
        unsafe {
            // Disable interrupts
            self_int_en.write_volatile(InterruptEnable::empty());

            // Enable DLAB
            self_line_ctrl.write_volatile(0x80);

            // Set maximum speed to 38400 bps by configuring DLL and DLM
            self_data.write_volatile(0x03);
            self_int_en.write_volatile(InterruptEnable::empty());

            // Disable DLAB and set data word length to 8 bits
            self_line_ctrl.write_volatile(0x03);

            // Enable FIFO, clear TX/RX queues and
            // set interrupt watermark at 14 bytes
            self_fifo_ctrl.write_volatile(0xC7);

            // Mark data terminal ready, signal request to send
            // and enable auxilliary output #2 (used as interrupt line for CPU)
            self_modem_ctrl.write_volatile(
                ModemControlRegister::REQUEST_TO_SEND
                    | ModemControlRegister::REQUEST_TO_SEND
                    | ModemControlRegister::OUT_1,
            );

            let _res = self_fifo_ctrl.read_volatile();

            plic::enable_interrupt(self.int_id);

            /*
            // Put into loopback mode to test the chip.
            self_modem_ctrl.write_volatile(
                ModemControlRegister::REQUEST_TO_SEND
                    | ModemControlRegister::OUT_1
                    | ModemControlRegister::OUT_2
                    | ModemControlRegister::LOOP,
            );

            const TEST_DATA: u8 = 0xAE;
            self_data.write_volatile(TEST_DATA);
            let read = self_data.read_volatile();
            if read != TEST_DATA {
                anyhow::bail!("ERROR Uart Loopback did not return test data.");
            }
            */
        }

        Ok(())
    }

    fn line_sts(&mut self) -> LineStsFlags {
        unsafe { LineStsFlags::from_bits_truncate(*self.line_sts.load(Ordering::Relaxed)) }
    }

    /// Sends a byte on the serial port.
    pub fn send(&mut self, data: u8) {
        let self_data = self.data.load(Ordering::Relaxed);
        unsafe {
            match data {
                8 | 0x7F => {
                    wait_for!(self.line_sts().contains(LineStsFlags::OUTPUT_EMPTY));
                    self_data.write(8);
                    wait_for!(self.line_sts().contains(LineStsFlags::OUTPUT_EMPTY));
                    self_data.write(b' ');
                    wait_for!(self.line_sts().contains(LineStsFlags::OUTPUT_EMPTY));
                    self_data.write(8)
                }
                _ => {
                    wait_for!(self.line_sts().contains(LineStsFlags::OUTPUT_EMPTY));
                    self_data.write(data);
                }
            }
        }
    }

    /// Receives a byte on the serial port.
    pub fn receive(&mut self) -> u8 {
        let self_data = self.data.load(Ordering::Relaxed);
        unsafe {
            wait_for!(self.line_sts().contains(LineStsFlags::INPUT_FULL));
            self_data.read()
        }
    }

    pub fn try_receive(&mut self) -> Option<u8> {
        let self_data = self.data.load(Ordering::Relaxed);
        unsafe {
            if self.line_sts().contains(LineStsFlags::INPUT_FULL) {
                Some(self_data.read_volatile())
            } else {
                None
            }
        }
    }
}

impl fmt::Write for MmioSerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.send(byte);
        }
        Ok(())
    }
}
