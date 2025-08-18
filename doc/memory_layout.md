# Memory Layout

|Address|Description|
|---:|:---|
|0x100000|Beginning of the kernel|
|0x400000|Beginning of the original kernel ELF file|
|0x600000|Beginning of the font file|
|0x700000|Beginning of the framebuffer info|


The beginning address of the kernel is controlled by using a linker script named `kernel/kernel.ld`, not by embedding it into the bootloader.
