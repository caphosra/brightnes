.PHONY: kernel bootloader run run-dbg gdb

kernel: ./kernel/Cargo.toml ./kernel/kernel.ld ./kernel/src/main.rs
	cargo build \
		--package brightnes-kernel \
		--target x86_64-unknown-none
	mkdir -p ./dest
	cp ./target/x86_64-unknown-none/debug/brightnes-kernel ./dest/kernel

bootloader: ./bootloader/Cargo.toml ./bootloader/src/main.rs
	cargo build \
		--package brightnes-bootloader \
		--target x86_64-unknown-uefi
	mkdir -p ./dest/efi/boot
	cp ./target/x86_64-unknown-uefi/debug/brightnes-bootloader.efi ./dest/efi/boot/bootx64.efi

run: kernel bootloader
	qemu-system-x86_64 -m 2G \
		-bios ./OVMF.fd \
		-drive format=raw,file=fat:rw:./dest

run-dbg: kernel bootloader
	qemu-system-x86_64 -m 2G \
		-bios ./OVMF.fd \
		-drive format=raw,file=fat:rw:./dest \
		-S -gdb tcp::31415

gdb:
	gdb -x ./.gdbinit
