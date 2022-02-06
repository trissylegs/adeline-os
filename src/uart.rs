use core::fmt::Debug;
use crate::traits::{Console, Driver};

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


#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum RegisterSize {
    Byte   = 1,
    Word   = 2,
    DWord  = 4,
}

#[derive(Debug)]
pub struct UartDriver {
    name: &'static str, 
    base: *mut u8,
    in_freq: u32,
    baudrate: u32,
    reg_shift: u32,
    reg_width: RegisterSize,
}

impl UartDriver {
    pub unsafe fn init(base: *mut u8, in_freq: u32, baudrate: u32, reg_shift: u32, reg_width: RegisterSize)
        -> UartDriver
    {
        let mut driver = UartDriver {
            name: "uart8250",
            base, in_freq, baudrate, reg_shift, reg_width,
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
        let offset = register_num << self.reg_shift;

        let addr = self.base.offset(offset as isize);
        write_at(addr, self.reg_width, value);
    }

    unsafe fn get_reg(&mut self, register_num: u32) -> u32 {
        let offset = register_num << self.reg_shift;
        let addr = self.base.offset(offset as isize);
        read_at(addr, self.reg_width)
    }
}

impl Driver for UartDriver {
    fn name(&self) -> &'static str {
        self.name
    }
}

impl Console for UartDriver {
    fn put_char(&mut self, value: u8) {
        loop {
            let lsr = unsafe { self.get_reg(UART_LSR_OFFSET) };
            if (lsr & UART_LSR_THRE) != 0 {
                break;
            }
        }

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


unsafe fn write_at(ptr: *mut u8, size: RegisterSize, value: u32) {
    match size {
        RegisterSize::Byte => ptr.write_volatile(value as u8),
        RegisterSize::Word => (ptr as *mut u16).write_volatile(value as u16),
        RegisterSize::DWord => (ptr as *mut u32).write_volatile(value),
    }
}

unsafe fn read_at(ptr: *mut u8, size: RegisterSize) -> u32 {
    match size {
        RegisterSize::Byte => ptr.read_volatile() as u32,
        RegisterSize::Word => (ptr as *mut u16).read_volatile() as u32,
        RegisterSize::DWord => (ptr as *mut u32).read_volatile(),
    }
}
