
export TARGET=riscv64gc-unknown-none-elf
export HOST_CC=gcc
export TARGET_CC=riscv64-elf-gcc

asm_files:=entry.S
build_rs:=build.rs
rust_files:=$(shell find src/ -type f -name '*.rs')

src_files=${asm_files} ${build_rs} ${rust_files}

.phony: all
all: ${src_files}
	cargo build --target=${TARGET}

entry.S:
	echo "entry.S"

run-gdb:
	qemu-system-riscv64 -machine virt -serial mon:stdio -gdb tcp::1234 -S -bios ../opensbi/build/platform/generic/firmware/fw_dynamic.elf -kernel target/riscv64gc-unknown-none-elf/debug/kernel

run:
	cargo run
