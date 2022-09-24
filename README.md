# Alleged Kernel in Rust for RISC-V 64.

## What works

1. Bootstraping rust runtime with QEMU -machine=virt (Which we shall call: _virt_)
2. Reading the device tree. We just pull out what we need in a big horrible function.
2. Basic Memory allocation with a fixed size heap.
3. UART serial console. With println! support. Fallback to SBI putchar in case of early panic.
4. Unit test framework.
5. Timers using the monotonic `mtime` clock.
6. Reading RTC time.
7. System reset/shutdown via SBI.
8. Easy launching by going `cargo run`. (Assuming you have a toolchain and qemu)

## What doesn't

1. Paging.
2. Good allocation of all the avalible memory.
3. External interrupts. Yes the code seems to be there for UART interrupts. But it doesn't work.
4. 

## How to:

### All os.

Assming `$SRC` is a directory
* Clone this repo as a subdirectory of `$SRC`:
```sh
$ git clone git@github.com:trissylegs/adeline-os.git "$SRC/kernel"
```
* Clone opensbi next to this repo:
```sh
$ git clone git@github.com:riscv-software-src/opensbi.git "$SRC/opensbi"
```

Ensure you have QEMU with RISC-V support. Checking:

```sh
$ qemu-system-riscv64 -version
QEMU emulator version 7.1.0
Copyright (c) 2003-2022 Fabrice Bellard and the QEMU Project developers
```

To Compile:

```sh
# There's no strict order. OpenSBI isn't a compile-time dependency.
$ make opensbi
$ cargo build
```

To run:

```
make run
```

or

```
cargo run
```

To debug open two terminals

```
# Terminal 1:
$ make run-gdb
```

```
# Terminal 2:
$ make attach-gdb
```



### Macos

*TODO*

Notes: 
* The brew tap for `riscv-gnu-toolchain` was outdated last I checked.
* I'm manually compiling and building the risc-v tools from: https://github.com/riscv-collab/riscv-gnu-toolchain
* Just clone the repo. Setup your desintion with write permissions for your user. I put them in `/opt/riscv`. 
* The tools just need to be in your `$PATH` under names like: `riscv64-unknown-elf-gcc`
* The rust instructions for linux should be correct still for mac.
* 


### Linux

FIXME: I broke linux config making macos work :/

1. Linux host (Maybe FreeBSD works)
2. qemu-system-riscv64 (A riscv64 emulator)
3. A gnu `riscv-elf-` toolchain. So can you do this:
```
$ riscv64-elf-gcc
riscv64-elf-gcc: fatal error: no input files
compilation terminated.
```
4. Rust + Cargo with target `riscv64gc-unknown-none-elf`. To install with rustup run. (I think this is optional, but it doesn't hurt)
```
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
