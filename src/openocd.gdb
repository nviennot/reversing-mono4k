source ./openocd-noload.gdb

# Write our program into the device's internal flash
load

# Resume execution
monitor resume
detach
quit
