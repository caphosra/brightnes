# Memory Layout

|Address|Description|
|---:|:---|
|0x100000|Beginning of the kernel|
|0x200000|Beginning of the original kernel ELF file|

The beginning address of the kernel is controlled by using a linker script named `kernel/kernel.ld`, not by embedding it into the bootloader.
