.PHONY: resources run run-dbg gdb

OUT_DIR = ./dest

QEMU_FLAGS = -m 2G -bios ./OVMF.fd \
	-drive format=raw,file=fat:rw:$(OUT_DIR)

KERNEL_SOURCES = ./kernel/Cargo.toml \
	./kernel/kernel.ld \
	./kernel/src/int/mod.rs \
	./kernel/src/int/pic.rs \
	./kernel/src/io/fs_driver.rs \
	./kernel/src/font.rs \
	./kernel/src/frame_buffer.rs \
	./kernel/src/main.rs \
	./kernel/src/mem.rs
BOOTLOADER_SOURCES = ./bootloader/Cargo.toml \
	./bootloader/src/elf.rs \
	./bootloader/src/frame_buffer.rs \
	./bootloader/src/fs.rs \
	./bootloader/src/main.rs

$(OUT_DIR)/kernel: $(KERNEL_SOURCES)
	cargo build \
		--package brightnes-kernel \
		--target x86_64-unknown-none
	mkdir -p $(OUT_DIR)
	cp ./target/x86_64-unknown-none/debug/brightnes-kernel $(OUT_DIR)/kernel

$(OUT_DIR)/efi/boot/bootx64.efi: $(BOOTLOADER_SOURCES)
	cargo build \
		--package brightnes-bootloader \
		--target x86_64-unknown-uefi
	mkdir -p $(OUT_DIR)/efi/boot
	cp ./target/x86_64-unknown-uefi/debug/brightnes-bootloader.efi $(OUT_DIR)/efi/boot/bootx64.efi

resources: ./res/font/Tamsyn8x16r.psf.gz
	mkdir -p $(OUT_DIR)/res/font
	gzip -d < ./res/font/Tamsyn8x16r.psf.gz > $(OUT_DIR)/system_font.psf

build: $(OUT_DIR)/kernel $(OUT_DIR)/efi/boot/bootx64.efi resources
	@echo "Build complete."

run: $(OUT_DIR)/kernel $(OUT_DIR)/efi/boot/bootx64.efi resources
	qemu-system-x86_64 $(QEMU_FLAGS)

run-dbg: $(OUT_DIR)/kernel $(OUT_DIR)/efi/boot/bootx64.efi resources
	qemu-system-x86_64 $(QEMU_FLAGS) \
		-S -gdb tcp::31415

gdb:
	gdb -x ./.gdbinit
