source ./openocd-noload.gdb

# Write our program into the device's internal flash
load
monitor reset

# Resume execution (attached)
# break DefaultHandler
# break HardFault
# break rust_begin_unwind
# continue

# Resume execution (detached)
monitor go
#monitor resume
#detach
quit
