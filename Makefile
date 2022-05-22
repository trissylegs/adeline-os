
export TARGET=riscv64gc-unknown-none-elf
export CROSS_COMPILE=riscv64-unknown-elf-

PLATFORM=generic

export CC=$(CROSS_COMPILE)gcc
export AR=$(CROSS_COMPILE)ar
export LD=$(CROSS_COMPILE)ld
export OBJCOPY=$(CROSS_COMPILE)13

FW_JUMP=y
FW_JUMP_ADDR=0x80200000

MAKE_OPENSBI=$(MAKE) PLATFORM=$(PLATFORM) CROSS_COMPILE=$(CROSS_COMPILE)

QEMU_MACHINE=virt
QEMU_MEMORY=128M

.phony: build clean run-gdb attach-gdb run
build:
	cargo build --target=${TARGET}

clean:
	cargo clean
	cd ../opensbi && $(MAKE_OPENSBI) clean

opensbi:
	cd ../opensbi && $(MAKE_OPENSBI)

run-gdb:
	qemu-system-riscv64 \
		-machine $(QEMU_MACHINE) \
		-m $(QEMU_MEMORY) \
		-serial mon:stdio \
		-gdb tcp::1234 -S \
		-bios ../opensbi/build/platform/generic/firmware/fw_jump.elf \
		-kernel target/riscv64gc-unknown-none-elf/debug/kernel

dump-dtb:
	qemu-system-riscv64 \
		-machine $(QEMU_MACHINE) \
		-m $(QEMU_MEMORY) \
		-machine dumpdtb=qemu-virt.dtb
	dtc -I dtb -O dts qemu-virt.dtb -o qemu-virt.dts

attach-gdb:
	riscv64-elf-gdb \
	        -ex 'file target/riscv64gc-unknown-none-elf/debug/kernel' \
		-ex 'add-symbol-file ../opensbi/build/platform/generic/firmware/fw_jump.elf' \
	        -ex 'target remote localhost:1234'
	killall qemu-system-riscv64
run:
	cargo run

