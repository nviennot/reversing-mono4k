# Connect to openocd
target extended-remote :3333

# Enable OpenOCD's semihosting capability
monitor arm semihosting enable
#monitor arm semihosting_fileio enable

# Set backtrace limit to not have infinite backtrace loops
set backtrace limit 32
set pagination off

# Print demangled symbols
set print asm-demangle on

# Loads the plugin to read registers in readable format
source ../repos/PyCortexMDebug/scripts/gdb.py
svd_load ../repos/gd32-rs/svd/gd32f307.svd


# Print 5 instructions every time we break.
# Note that `layout asm` is also pretty good, but my up arrow doesn't work
# anymore in this mode, so I prefer display/5i.
display/5i $pc

define count_instr_until
  set $count=0
  while ($pc != $arg0)
    stepi
    set $count=$count+1
  end
  print $count
end

# # detect unhandled exceptions, hard faults and panics
# 
# #break DefaultHandler
# #break HardFault
# #break rust_begin_unwind
# 
# # # run the next few lines so the panic message is printed immediately
# # # the number needs to be adjusted for your panic handler
# # commands $bpnum
# # next 4
# # end
# 
# # *try* to stop at the user entry point (it might be gone due to inlining)
# #break main
# 
# monitor arm semihosting enable
# monitor arm semihosting_fileio enable
# 
# # # send captured ITM to the file itm.fifo
# # # (the microcontroller SWO pin must be connected to the programmer SWO pin)
# # # 8000000 must match the core clock frequency
# # monitor tpiu config internal itm.txt uart off 8000000
# 
# # # OR: make the microcontroller SWO pin output compatible with UART (8N1)
# # # 8000000 must match the core clock frequency
# # # 2000000 is the frequency of the SWO pin
# # monitor tpiu config external uart off 8000000 2000000
# 
# # # enable ITM port 0
# # monitor itm port 0 on
# 
#
