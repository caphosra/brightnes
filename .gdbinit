target remote :31415
file dest/kernel
b kernel_main
b brightnes_kernel::int::Interrupt::init
hb brightnes_kernel::int::double_fault_handler
hb brightnes_kernel::int::general_protection_fault_handler
hb brightnes_kernel::int::page_fault_handler
