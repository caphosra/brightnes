# Memory Layout

|Address|Description|
|---:|:---|
|0x1_000_000|Beginning of the font file|
|0x2_000_000|Beginning of the framebuffer info|
|0x3_000_000|Beginning of the NES file|
|0x4_000_000|Beginning of the heap|
|0x6_000_000|End of the heap|
|0x8_000_000|Beginning of the kernel|

The beginning address of the kernel is controlled by using a linker script named `kernel/kernel.ld`, not by embedding it into the bootloader.
