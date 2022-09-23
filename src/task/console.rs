use crossbeam_queue::ArrayQueue;
use spin::Once;

use crate::prelude::*;

static UART_QUEUE: Once<ArrayQueue<u8>> = Once::INIT;

pub fn add_byte(byte: u8) {
    if let Some(queue) = UART_QUEUE.get() {
        if let Err(_) = queue.push(byte) {
            println!("WARNING: serial queue full; dropping input");
        }
    } else {
        println!("WARNING: serial queu uninitialized");
    }
}
