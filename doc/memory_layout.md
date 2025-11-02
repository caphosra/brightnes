# Memory Layout

|Address|Description|
|---:|:---|
|0x200_0000|Beginning of the font file|
|0x280_0000|Beginning of the framebuffer info|
|0x300_0000|Beginning of the NES file|
|0x680_0000|End of the info stack|
|0x700_0000|End of the monitor stack|
|0x780_0000|End of the log stack|
|0x800_0000|Beginning of the kernel|
|0x1000_0000|Beginning of the heap|
|0x3000_0000|End of the heap|
|0x4000_0000|End of the main stack|
|0x5000_0000|End of the game stack|

The beginning address of the kernel is controlled by using a linker script named `kernel/kernel.ld`, not by embedding it into the bootloader.
