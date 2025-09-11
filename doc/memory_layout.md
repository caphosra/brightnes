# Memory Layout

|Address|Description|
|---:|:---|
|0x0200_0000|Beginning of the font file|
|0x0280_0000|Beginning of the framebuffer info|
|0x0300_0000|Beginning of the NES file|
|0x0400_0000|Beginning of the heap|
|0x1400_0000|End of the heap|
|0x2000_0000|Beginning of the kernel|

The beginning address of the kernel is controlled by using a linker script named `kernel/kernel.ld`, not by embedding it into the bootloader.
