
export TARGET=riscv64gc-unknown-none-elf
export CROSS_COMPILE=riscv64-unknown-elf-

PLATFORM=generic

export CC=$(CROSS_COMPILE)gcc
export AR=$(CROSS_COMPILE)ar
export LD=$(CROSS_COMPILE)ld
export OBJCOPY=$(CROSS_COMPILE)13

RAM_BASE = 0x80000000
JUMP_OFF =    0x80000

FW_JUMP=y
FW_JUMP_ADDR=0x80080000

MAKE_OPENSBI=$(MAKE) PLATFORM=$(PLATFORM) CROSS_COMPILE=$(CROSS_COMPILE) FW_JUMP=$(FW_JUMP) FW_JUMP_ADDR=$(FW_JUMP_ADDR)

QEMU_MACHINE=virt
QEMU_MEMORY=1G
QEMU_SMP=1

.phony: build clean run-gdb attach-gdb run
build:
	cargo build --target=${TARGET}

clean:
	cargo clean
	cd ../opensbi && $(MAKE_OPENSBI) clean

opensbi:
	cd ../opensbi && $(MAKE_OPENSBI)

run:
	cargo build
	cat /dev/zero | pv -q -L 3 | qemu-system-riscv64 \
		-machine $(QEMU_MACHINE) \
		-m $(QEMU_MEMORY) \
		-smp $(QEMU_SMP) \
		-serial mon:stdio \
		-d int -D log.txt \
		-bios ../opensbi/build/platform/generic/firmware/fw_jump.elf \
		-kernel target/riscv64gc-unknown-none-elf/debug/kernel

run-gdb:
	cargo build
	cat /dev/zero | pv -q -L 3 | qemu-system-riscv64 \
		-machine $(QEMU_MACHINE) \
		-m $(QEMU_MEMORY) \
		-smp $(QEMU_SMP) \
		-serial mon:stdio \
		-d int -D log.txt \
		-gdb tcp::1234 -S \
		-bios ../opensbi/build/platform/generic/firmware/fw_jump.elf \
		-kernel target/riscv64gc-unknown-none-elf/debug/kernel

dump-dtb:
	qemu-system-riscv64 \
		-machine $(QEMU_MACHINE) \
		-m $(QEMU_MEMORY) \
		-smp $(QEMU_SMP) \
		-machine dumpdtb=qemu-virt.dtb
	dtc -I dtb -O dts qemu-virt.dtb -o qemu-virt.dts

attach-gdb:
	riscv64-unknown-elf-gdb \
		-ex 'file target/riscv64gc-unknown-none-elf/debug/kernel' \
		-ex 'add-symbol-file ../opensbi/build/platform/generic/firmware/fw_jump.elf' \
		-ex 'target remote localhost:1234'
	killall qemu-system-riscv64

