use core::{fmt::{Debug, Write}, mem::size_of};

use crate::{traits::{Console, Driver}, MemoryRange};

const UART_RBR_OFFSET: u32 = 0;	/* In:  Recieve Buffer Register */
const UART_THR_OFFSET: u32 = 0;	/* Out: Transmitter Holding Register */
const UART_DLL_OFFSET: u32 = 0;	/* Out: Divisor Latch Low */
const UART_IER_OFFSET: u32 = 1;	/* I/O: Interrupt Enable Register */
const UART_DLM_OFFSET: u32 = 1;	/* Out: Divisor Latch High */
const UART_FCR_OFFSET: u32 = 2;	/* Out: FIFO Control Register */
const UART_IIR_OFFSET: u32 = 2;	/* I/O: Interrupt Identification Register */
const UART_LCR_OFFSET: u32 = 3;	/* Out: Line Control Register */
const UART_MCR_OFFSET: u32 = 4;	/* Out: Modem Control Register */
const UART_LSR_OFFSET: u32 = 5;	/* In:  Line Status Register */
const UART_MSR_OFFSET: u32 = 6;	/* In:  Modem Status Register */
const UART_SCR_OFFSET: u32 = 7;	/* I/O: Scratch Register */
const UART_MDR1_OFFSET: u32 = 8;	/* I/O:  Mode Register */

const UART_LSR_FIFOE: u32          = 0x80; /* Fifo error */
const UART_LSR_TEMT: u32           = 0x40; /* Transmitter empty */
const UART_LSR_THRE: u32           = 0x20; /* Transmit-hold-register empty */
const UART_LSR_BI: u32             = 0x10; /* Break interrupt indicator */
const UART_LSR_FE: u32             = 0x08; /* Frame error indicator */
const UART_LSR_PE: u32             = 0x04; /* Parity error indicator */
const UART_LSR_OE: u32             = 0x02; /* Overrun error indicator */
const UART_LSR_DR: u32             = 0x01; /* Receiver data ready */
const UART_LSR_BRK_ERROR_BITS: u32 = 0x1E; /* BI, FE, PE, OE bits */

#[derive(Debug)]
pub struct UartDriver {
    config: Config,
    base: &'static mut [u64],
    in_freq: u32,
    baudrate: u32,
}

#[derive(Debug)]
pub struct Config {
    pub name: &'static str,
    pub interrupts: u32,
    pub interrupt_parent: u32,
    pub clock_frequency: u32,
    pub reg: MemoryRange,
    pub compatible: &'static str,
}

impl UartDriver {
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub unsafe fn init(config: Config)
        -> UartDriver
    {
        let base = core::slice::from_raw_parts_mut(
            config.reg.base as *mut u64, 
            config.reg.size / (size_of::<u64>()) );

        let in_freq = config.clock_frequency;
        let baudrate = 115200;

        let mut driver = UartDriver {
            config,
            base,
            in_freq,
            baudrate,
        };

        let bdiv = in_freq / (16 * baudrate);

        // No interrupts.
        driver.set_reg(UART_IER_OFFSET, 0x00);
        // Enable DLAB
        driver.set_reg(UART_LCR_OFFSET, 0x80);

        if bdiv > 0 {
            // Divisor low
            driver.set_reg(UART_DLL_OFFSET, bdiv & 0xff);
            // Divisor high
            driver.set_reg(UART_DLM_OFFSET, (bdiv >> 8) & 0xff);
        }

        // 8 bits, no parity, one stop bit
        driver.set_reg(UART_LCR_OFFSET, 0x03);
        // Enable FIFO
        driver.set_reg(UART_FCR_OFFSET, 0x01);
        // No modem control DTR RTS
        driver.set_reg(UART_MCR_OFFSET, 0x00);
        // Clear line status
        driver.get_reg(UART_LSR_OFFSET);
        // Read receive buffer
        driver.get_reg(UART_RBR_OFFSET);

        driver.set_reg(UART_SCR_OFFSET, 0x00);

        driver
    }
    
    unsafe fn set_reg(&mut self, register_num: u32, value: u32) {
        let addr = &mut self.base[register_num as usize];
        *addr = value as u64;
    }

    unsafe fn get_reg(&mut self, register_num: u32) -> u32 {
        let addr = &mut self.base[register_num as usize];
        *addr as u32        
    }

    fn wait(&mut self) {
        loop {
            let lsr = unsafe { self.get_reg(UART_LSR_OFFSET) };
            if (lsr & UART_LSR_THRE) != 0 {
                break;
            }
        }
    }
}

impl Driver for UartDriver {
    fn name(&self) -> &'static str {
        "uart"
    }
}

impl Console for UartDriver {

    fn put_char(&mut self, value: u8) {
        self.wait();
        unsafe {
            self.set_reg(UART_THR_OFFSET, value as u32);
        }
    }

    fn get_char(&mut self) -> Option<u8> {
        let lsr = unsafe { self.get_reg(UART_LSR_OFFSET) };
        if (lsr & UART_LSR_DR) != 0 {
            unsafe { Some(self.get_reg(UART_RBR_OFFSET) as u8) }
        } else {
            None
        }
    }

}

impl Write for UartDriver {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            self.put_char(b);
        }
        Ok(())
    }
}
