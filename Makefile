.PHONY: build run run-dbg clean gdb

OUT_DIR = ./dest

BOOTLOADER_CARGO_FLAGS = --release
KERNEL_CARGO_FLAGS = --release

QEMU_FLAGS =

QEMU_DEFAULT_FLAGS = -m 2G -bios ./OVMF.fd \
	-drive format=raw,file=fat:rw:$(OUT_DIR),index=0 \
	-drive format=raw,if=none,id=main_drive,file=./disk.img,index=1 \
	-device virtio-blk-pci,drive=main_drive \
	-device virtio-sound-pci \
	$(QEMU_FLAGS)

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
	./kernel/src/serial.rs \
	./kernel/src/system.rs
BOOTLOADER_SOURCES = ./bootloader/Cargo.toml \
	./bootloader/src/elf.rs \
	./bootloader/src/frame_buffer.rs \
	./bootloader/src/fs.rs \
	./bootloader/src/main.rs

$(OUT_DIR)/brightnes-kernel: $(KERNEL_SOURCES) $(COMMON_SOURCES)
	mkdir -p $(OUT_DIR)
	cargo build \
		-Zunstable-options \
		--package brightnes-kernel \
		--target x86_64-unknown-none \
		--artifact-dir $(OUT_DIR) \
		$(KERNEL_CARGO_FLAGS)

$(OUT_DIR)/efi/boot/bootx64.efi: $(BOOTLOADER_SOURCES)
	mkdir -p $(OUT_DIR)/efi/boot
	cargo build \
		-Zunstable-options \
		--package brightnes-bootloader \
		--target x86_64-unknown-uefi \
		--artifact-dir $(OUT_DIR)/efi/boot \
		$(BOOTLOADER_CARGO_FLAGS)
	mv $(OUT_DIR)/efi/boot/brightnes-bootloader.efi $(OUT_DIR)/efi/boot/bootx64.efi

$(OUT_DIR)/system_font.psf: ./res/font/Tamsyn8x16r.psf.gz
	mkdir -p $(OUT_DIR)
	gzip -d < ./res/font/Tamsyn8x16r.psf.gz > $(OUT_DIR)/system_font.psf

disk.img:
	dd if=/dev/zero of=disk.img bs=1M count=128
	mkfs.fat -F32 -n BRIGHTNES disk.img
	mmd -i ./disk.img ::nes
	mcopy -i ./disk.img ./res/nes/*.nes ::nes
	mcopy -i ./disk.img ./res/nes/*.txt ::nes

build: $(OUT_DIR)/brightnes-kernel $(OUT_DIR)/efi/boot/bootx64.efi $(OUT_DIR)/system_font.psf
	@echo "Build complete."

run: $(OUT_DIR)/brightnes-kernel $(OUT_DIR)/efi/boot/bootx64.efi $(OUT_DIR)/system_font.psf
	qemu-system-x86_64 $(QEMU_DEFAULT_FLAGS)

run-dbg: $(OUT_DIR)/brightnes-kernel $(OUT_DIR)/efi/boot/bootx64.efi $(OUT_DIR)/system_font.psf
	qemu-system-x86_64 $(QEMU_DEFAULT_FLAGS) \
		-S -gdb tcp::31415

clean:
	cargo clean
	rm -rf $(OUT_DIR)
	@echo "Clean complete."

gdb:
	gdb -x ./.gdbinit
