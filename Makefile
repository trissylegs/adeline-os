
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
	echo "WOOT"

run:
	cargo run --target=${TARGET}

