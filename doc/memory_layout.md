# Memory Layout

|Address|Description|
|---:|:---|
|0x400_000|Beginning of the original kernel ELF file|
|0x600_000|Beginning of the font file|
|0x700_000|Beginning of the framebuffer info|
|0xF_000_000|Beginning of the kernel|

The beginning address of the kernel is controlled by using a linker script named `kernel/kernel.ld`, not by embedding it into the bootloader.
