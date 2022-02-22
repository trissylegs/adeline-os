# Alleged Kernel in Rust for RISC-V 64.

## What works

1. Bootstraping rust runtime with QEMU -machine=virt (Which we shal call: _virt_)
2. SBI v0.1 putchar() extension for println!()
3. SBI v0.1 shutdown extension.
4. Unit test framework.
5. Memory allocation with a fixed size heap set by BSS section.
6. Basic interrupts/exception handling. On same stack.
7. Easy launching by going `cargo run`

## What doesn't

1. The UART driver on _virt_. I think OpenSBI uses PMP to block access to UART. 
   Because writing to UART registers will trigger a Memory protection fault.
2. Paging. (Current WIP)
3. Parsing Device Trees. (It's tedious so I just hardcoded _virt_ with default settings).


## How to:

### You need

1. Linux host (Maybe FreeBSD works)
2. qemu-system-riscv64 (A riscv64 emulator)
3. A gnu `riscv-elf-` toolchain. So can you do this:
```
$ riscv64-elf-gcc
riscv64-elf-gcc: fatal error: no input files
compilation terminated.
```
4. Rust + Cargo with target `riscv64gc-unknown-none-elf`. To install with rustup run. (I think this is optional, but it doesn't hurt)
```sh
$ rustup target add riscv64gc-unknown-none-elf
```
4. _Optional_: `riscv-elf-gdb` (For debugging)

### Running.

1. Make a directory for working with multiple repos.
```
$ mkdir adeline-os
$ cd adeline-os
```
2. Checkout this repo
```
$ git checkout git@github.com:trissylegs/adeline-os.git
```
3. Checkout [OpenSBI](https://github.com/riscv-software-src/opensbi).
```
$ git checkout git@github.com:riscv-software-src/opensbi.git
```
4. Make opensbi
```
$ cd opensbi
$ make CROSS_COMPILE=riscv64-elf- PLATFORM=generic
```
5. Go back to our repo
```
$ cd ../adeline-os
```
6. Cargo run
```
$ cargo run
```

Should get something like this:
```
OpenSBI v1.0-5-g5d025eb
   ____                    _____ ____ _____
  / __ \                  / ____|  _ \_   _|
 | |  | |_ __   ___ _ __ | (___ | |_) || |
 | |  | | '_ \ / _ \ '_ \ \___ \|  _ < | |
 | |__| | |_) |  __/ | | |____) | |_) || |_
  \____/| .__/ \___|_| |_|_____/|____/_____|
        | |
        |_|

Platform Name             : riscv-virtio,qemu
...
Boot HART MEDELEG         : 0x000000000000b109
Hello, world
heart: 0
device tree: 0x87000000
Sstatus {
  uie: false
  sie: false
  upie: false
  spie: false
  spp: User
  fs: Dirty
  xs: Off
  sum: false
  mxr: false
  sd: true
}
Sstatus {
  usoft: false
  ssoft: false
  utimer: false
  utimer: false
  stimer: false
  uext: false
  sext: false
}
Stvec {
  address: 80204000
  mode: Some(Direct)
}

Bare
0
0
Base mapping not more details.
```


## Debugging

After doing above. 

1. Open two terminals in the root of this repos source.
2. Launch qemu with kernel. It'll pause on the first instruction (should be at 0x1000)
```
$ make run-gdb
```
3. Launch gdb atteched to remote session
```
$ make attach-gdb
```
4. Set your breakpoints. Some good breakpoints:
    * `break *0x80000000` Entrypoint for OpenSBI. You can also get here quickly by just using `stepi`.
    * `break sbi_trap_handler`. Entry point for SBI's trap handler. You'll get here when you do something wrong, make an SBI call, or when a timer goes off.
    * `break *0x80200000`. Entry for Supervisor mode. Assembly defined in `entry.S`. Address hardcoded in linker.ld
    * `break kmain`. Rust entrypoint.
