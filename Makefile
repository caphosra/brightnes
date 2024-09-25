bootloader:
	cargo build --target x86_64-unknown-uefi
	mkdir -p ./dest/efi/boot
	cp ./target/x86_64-unknown-uefi/debug/brightnes.efi ./dest/efi/boot/bootx64.efi

qemu:
	qemu-system-x86_64 -m 2G \
		-bios ./OVMF.fd \
		-drive format=raw,file=fat:rw:./dest
