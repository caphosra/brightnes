.PHONY: build-kernel build-kernel-dbg resources build run run-dbg gdb

OUT_DIR = ./dest
NES_FILE = ./res/nes/$(GAME)
SERIAL_PORT = 19837

QEMU_FLAGS = -m 2G -bios ./OVMF.fd \
	-drive format=raw,file=fat:rw:$(OUT_DIR) \
	-monitor stdio \
	-serial tcp::$(SERIAL_PORT),server,nowait

KERNEL_SOURCES = ./kernel/Cargo.toml \
	./kernel/kernel.ld \
	./kernel/src/int/keyboard.rs \
	./kernel/src/int/mod.rs \
	./kernel/src/nes/apu/bus.rs \
	./kernel/src/nes/apu/mod.rs \
	./kernel/src/nes/cartridge/mapper0.rs \
	./kernel/src/nes/cartridge/mapper2.rs \
	./kernel/src/nes/cartridge/mapper3.rs \
	./kernel/src/nes/cartridge/mapper4.rs \
	./kernel/src/nes/cartridge/mod.rs \
	./kernel/src/nes/cpu/bus.rs \
	./kernel/src/nes/cpu/instr.rs \
	./kernel/src/nes/cpu/mod.rs \
	./kernel/src/nes/cpu/ram.rs \
	./kernel/src/nes/ppu/bus.rs \
	./kernel/src/nes/ppu/color.rs \
	./kernel/src/nes/ppu/mod.rs \
	./kernel/src/nes/ppu/oam.rs \
	./kernel/src/nes/ppu/vram.rs \
	./kernel/src/nes/mod.rs \
	./kernel/src/nes/pad.rs \
	./kernel/src/font.rs \
	./kernel/src/frame_buffer.rs \
	./kernel/src/info.rs \
	./kernel/src/logger.rs \
	./kernel/src/main.rs \
	./kernel/src/mem.rs \
	./kernel/src/proc.rs \
	./kernel/src/serial.rs
BOOTLOADER_SOURCES = ./bootloader/Cargo.toml \
	./bootloader/src/elf.rs \
	./bootloader/src/frame_buffer.rs \
	./bootloader/src/fs.rs \
	./bootloader/src/main.rs

KERNEL_RELEASE = ./target/x86_64-unknown-none/release/brightnes-kernel
KERNEL_DEBUG = ./target/x86_64-unknown-none/debug/brightnes-kernel

$(KERNEL_RELEASE): $(KERNEL_SOURCES)
	cargo build \
		--package brightnes-kernel \
		--target x86_64-unknown-none \
		--release

build-kernel: $(KERNEL_RELEASE)
	mkdir -p $(OUT_DIR)
	cp $(KERNEL_RELEASE) $(OUT_DIR)/kernel

$(KERNEL_DEBUG): $(KERNEL_SOURCES)
	cargo build \
		--package brightnes-kernel \
		--target x86_64-unknown-none

build-kernel-dbg: $(KERNEL_DEBUG)
	mkdir -p $(OUT_DIR)
	cp $(KERNEL_DEBUG) $(OUT_DIR)/kernel

$(OUT_DIR)/efi/boot/bootx64.efi: $(BOOTLOADER_SOURCES)
	cargo build \
		--package brightnes-bootloader \
		--target x86_64-unknown-uefi
	mkdir -p $(OUT_DIR)/efi/boot
	cp ./target/x86_64-unknown-uefi/debug/brightnes-bootloader.efi $(OUT_DIR)/efi/boot/bootx64.efi

resources: ./res/font/Tamsyn8x16r.psf.gz
	mkdir -p $(OUT_DIR)/res/font
	gzip -d < ./res/font/Tamsyn8x16r.psf.gz > $(OUT_DIR)/system_font.psf

	cp $(NES_FILE) $(OUT_DIR)/game.nes

build: build-kernel $(OUT_DIR)/efi/boot/bootx64.efi resources
	@echo "Build complete."

run: build-kernel $(OUT_DIR)/efi/boot/bootx64.efi resources
	qemu-system-x86_64 $(QEMU_FLAGS)

run-dbg: build-kernel-dbg $(OUT_DIR)/efi/boot/bootx64.efi resources
	qemu-system-x86_64 $(QEMU_FLAGS) \
		-S -gdb tcp::31415

gdb:
	gdb -x ./.gdbinit
