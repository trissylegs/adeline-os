
# Target for Cargo/rustc.
export TARGET=riscv64gc-unknown-none-elf
# Prefix for toolchain
export CROSS_COMPILE=riscv64-unknown-elf-

# Toolchain binaries
export CC=$(CROSS_COMPILE)gcc
export AR=$(CROSS_COMPILE)ar
export LD=$(CROSS_COMPILE)ld
export OBJCOPY=$(CROSS_COMPILE)objcopy

# Memory offsets for qemu virt.
RAM_BASE = 0x80000000
# 2^19. OpenSBI will reserve this much memory at the start of ram. So we can start right after that.
JUMP_OFF =    0x80000

# OpenSBI platform.
PLATFORM=generic
# Use the fw_jump image. This is the simplest and seems to match what real hardware does. (When u-boot isn't used)
FW_JUMP=y
# RAM_BASE + JUMP_OFF
FW_JUMP_ADDR=0x80080000

# Make command for opensbi.
MAKE_OPENSBI=$(MAKE) PLATFORM=$(PLATFORM) CROSS_COMPILE=$(CROSS_COMPILE) FW_JUMP=$(FW_JUMP) FW_JUMP_ADDR=$(FW_JUMP_ADDR)

QEMU_MACHINE=virt
QEMU_MEMORY=128M
# Yes, it does run with multiple cores present. But it doesn't do much with it.
QEMU_SMP=1



.phony: build clean run run-gdb attach-gdb
build:
	cargo build

clean:
	cargo clean
	cd ../opensbi && $(MAKE_OPENSBI) clean

opensbi:
	cd ../opensbi && $(MAKE_OPENSBI)

run:	
	qemu-system-riscv64 \
		-machine $(QEMU_MACHINE) \
		-m $(QEMU_MEMORY) \
		-smp $(QEMU_SMP) \
		-serial mon:stdio \
		-d int -D log.txt \
		-bios ../opensbi/build/platform/generic/firmware/fw_jump.elf \
		-kernel target/riscv64gc-unknown-none-elf/debug/kernel

run-gdb:	
	qemu-system-riscv64 \
		-machine $(QEMU_MACHINE) \
		-m $(QEMU_MEMORY) \
		-smp $(QEMU_SMP) \
		-serial mon:stdio \
		-d int -D log.txt \
		-gdb tcp::1234 -S \
		-bios ../opensbi/build/platform/generic/firmware/fw_jump.elf \
		-kernel target/riscv64gc-unknown-none-elf/debug/kernel

tail-interrupts:
	tail -F log.txt

dump-dtb:
	@ echo "*** WARNING ***"
	@ echo "The version this dumps is what OpenSBI sees. Not what the kernel sees."
	@ echo "OpenSBI will mask out Machine interrupts from PLIC, and add a memory reserved node"
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

