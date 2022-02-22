
export TARGET=riscv64gc-unknown-none-elf
export CROSS_COMPILE=riscv64-elf-
export TARGET_CC=$(CROSS_COMPILE)-gcc

.phony: all
all:
	cargo build --target=${TARGET}

run-gdb:
	qemu-system-riscv64 \
		-machine virt \
		-serial mon:stdio \
		-gdb tcp::1234 -S \
		-bios ../opensbi/build/platform/generic/firmware/fw_dynamic.elf \
		-kernel target/riscv64gc-unknown-none-elf/debug/kernel

attach-gdb:
	riscv64-elf-gdb \
        -ex 'file target/riscv64gc-unknown-none-elf/debug/kernel' \
        -ex 'add-symbol-file ../opensbi/build/platform/generic/firmware/fw_dynamic.elf' \
        -ex 'target remote localhost:1234'

run:
	cargo run

