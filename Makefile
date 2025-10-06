.PHONY: build-kernel \
	build-kernel-dbg \
	create-disk \
	resources build run run-dbg gdb

OUT_DIR = ./dest
NES_FILE = ./res/nes/$(GAME)

CARGO_FLAGS =
QEMU_FLAGS = -m 2G -bios ./OVMF.fd \
	-drive format=raw,file=fat:rw:$(OUT_DIR),index=0 \
	-drive format=raw,if=none,id=main_drive,file=./disk.img,index=1 \
	-device virtio-blk-pci,drive=main_drive \
	-device virtio-sound-pci \
	-monitor stdio

COMMON_SOURCES = ./common/Cargo.toml \
	./common/src/lib.rs \
	./common/src/serial.rs

KERNEL_SOURCES = ./kernel/Cargo.toml \
	./kernel/kernel.ld \
	./kernel/src/drivers/virtio/block.rs \
	./kernel/src/drivers/virtio/mod.rs \
	./kernel/src/drivers/virtio/sound.rs \
	./kernel/src/drivers/mod.rs \
	./kernel/src/drivers/pci.rs \
	./kernel/src/int/keyboard.rs \
	./kernel/src/int/mod.rs \
	./kernel/src/nes/apu/bus.rs \
	./kernel/src/nes/apu/dmc.rs \
	./kernel/src/nes/apu/mod.rs \
	./kernel/src/nes/apu/noise.rs \
	./kernel/src/nes/apu/pulse.rs \
	./kernel/src/nes/apu/triangle.rs \
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
	./kernel/src/fs.rs \
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

$(KERNEL_RELEASE): $(KERNEL_SOURCES) $(COMMON_SOURCES)
	cargo build \
		--package brightnes-kernel \
		--target x86_64-unknown-none \
		--release \
		$(CARGO_FLAGS)

build-kernel: $(KERNEL_RELEASE)
	mkdir -p $(OUT_DIR)
	cp $(KERNEL_RELEASE) $(OUT_DIR)/kernel

$(KERNEL_DEBUG): $(KERNEL_SOURCES) $(COMMON_SOURCES)
	cargo build \
		--package brightnes-kernel \
		--target x86_64-unknown-none \
		$(CARGO_FLAGS)

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
	gzip -d < ./res/font/Tamsyn8x16r.psf.gz > $(OUT_DIR)/system_font.psf

	cp $(NES_FILE) $(OUT_DIR)/game.nes

build: build-kernel $(OUT_DIR)/efi/boot/bootx64.efi resources
	@echo "Build complete."

create-disk:
	dd if=/dev/zero of=disk.img bs=1M count=128
	mkfs.fat -F32 -n BRIGHTNES disk.img

run: build-kernel $(OUT_DIR)/efi/boot/bootx64.efi resources
	qemu-system-x86_64 $(QEMU_FLAGS)

run-dbg: build-kernel-dbg $(OUT_DIR)/efi/boot/bootx64.efi resources
	qemu-system-x86_64 $(QEMU_FLAGS) \
		-S -gdb tcp::31415

gdb:
	gdb -x ./.gdbinit
