source ./openocd-noload.gdb

# Write our program into the device's internal flash
load

# break DefaultHandler
# break HardFault
# break rust_begin_unwind

# Resume execution
# continue
monitor resume
detach
quit
