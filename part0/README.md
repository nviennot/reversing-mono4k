Discovery of the hardware and firmware extraction
==================================================

## Hardware introduction

The controller board is shown as below.
The board of the Mono 4K is very similar to the Mono:
https://bleughbleugh.wordpress.com/2020/10/12/anycubic-photon-mono-teardown-part-1/

![Controller Board](controller_board.jpg)

A couple of important elements:
* GigaDevice GD32F307VET6 Arm Cortex M4 MCU. It has 512KB of internal flash, and 96KB of RAM.
  [Datasheet](http://www.gd32mcu.com/data/documents/shujushouce/GD32F307xx_Datasheet_Rev1.2.pdf)
  [User Manual](https://www.gigadevice.com/manual/gd32f307xxxx-user-manual/)
* Anlogic FPGA EF2L45LG144B (Most likely, should be under the gray heat sink).
* Winbond 25Q128JVSQ 16MB flash SPI chip.
  [Datasheet](http://www.winbond.com/resource-files/w25q128jv%20spi%20revc%2011162016.pdf)
* Winbond W9864G6KH-6 32MB SDRAM chip. [Datasheet](https://www.winbond.com/resource-files/w9864g6kh_a02.pdf)

## Connecting to the ARM MCU

Before attempting writing a new firmware for the unit, it's going to be helpful
to see what they are doing. We'll need to understand how to update the firmware
from the regular way so that we can have users trying out our new firmware.

Anycubic does not provide a firmware for the Mono 4K at this time. (See on the
official anycubic website
[here](https://www.anycubic.com/blogs/news/all-you-need-to-know-about-photon-mono-4k)).
We're going to have to extract it. The hope is to connect via debug protocol to
the MCU. Luckily, there's a nice little header on the top left of the board
looking like like an ARM Serial Wire Debug (SWD) header.

![Debug Header](debug_header.jpg)

The silk screen on the PCB shows `3V3, SLK, DIO, _, GND`. The non-specified wire
might be the reset pin.

I connected my [J-Link](https://www.segger.com/products/debug-probes/j-link/)
probe to the header, and running the following shows:

```
» JLinkExe -AutoConnect 1 -Device GD32F307VE -If SWD -Speed 4000

SEGGER J-Link Commander V7.60b (Compiled Dec 22 2021 12:50:26)
DLL version V7.60b, compiled Dec 22 2021 12:50:19

Connecting to J-Link via USB...O.K.
Firmware: J-Link V10 compiled Nov  2 2021 12:14:50
Hardware version: V10.10
VTref=3.300V (fixed)
Device "GD32F307VE" selected.


Connecting to target via SWD
Found SW-DP with ID 0x2BA01477
DPIDR: 0x2BA01477
CoreSight SoC-400 or earlier
Scanning AP map to find all available APs
AP[1]: Stopped AP scan as end of AP map has been reached AP[0]: AHB-AP (IDR: 0x24770011)
Iterating through AP map to find AHB-AP to use
AP[0]: Core found
AP[0]: AHB-AP ROM base: 0xE00FF000
CPUID register: 0x410FC241. Implementer code: 0x41 (ARM)
Found Cortex-M4 r0p1, Little endian.
FPUnit: 6 code (BP) slots and 2 literal slots
CoreSight components:
ROMTbl[0] @ E00FF000
[0][0]: E000E000 CID B105E00D PID 000BB00C SCS-M7
[0][1]: E0001000 CID B105E00D PID 003BB002 DWT
[0][2]: E0002000 CID B105E00D PID 002BB003 FPB
[0][3]: E0000000 CID B105E00D PID 003BB001 ITM
[0][4]: E0040000 CID B105900D PID 000BB9A1 TPIU
[0][5]: E0041000 CID 00000000 PID 00000000 ???
Cortex-M4 identified.
J-Link>
```

Wonderful! We get to reach the MCU.

Note that I tried using Open-OCD, with the following configuration, but I didn't
get far with it as the documentation is rather cryptic.

```
adapter driver jlink
adapter speed 4000
transport select swd

set _CHIPNAME gd32f307
set _CPUTAPID 0x1000563d

swd newdap $_CHIPNAME cpu -irlen 5 -expected-id $_CPUTAPID
dap create $_CHIPNAME.dap -chain-position $_CHIPNAME.cpu
target create $_CHIPNAME.cpu cortex_m -dap gd32f307.dap
```

## Dumping the firmware

The Anycubic Mono 4K controller board is a derivated board from Chitu systems.
You can see boards they are offering [here](https://shop.chitusystems.com/product-category/).

In the [download section](https://shop.chitusystems.com/download/), they show an
FPGA firmware, a core firmware, and a UI firmware.

Rigth now, we are after the core firmware, which sits in the MCU, and I couldn't
find a firmware to download for the Mono 4K.

With the JLink software, it's rather straightforward with the `savebin` command:

```
J-Link> savebin mcu.bin 0 0x80000
Opening binary file for writing... [mcu.bin]
Reading 524288 bytes from addr 0x00000000 into file...O.K.
```

After 1.3s, we have the MCU firmware on disk.

## Quick analysis

Let's look at the firmware on [binvis](https://binvis.io/).

![binvis](binvis.png)

It looks right (not encrypted, nor all blank).

We can look at the strings that are contained in this binary. 

```
» strings mcu.bin | sort -u

[...cut...]
R_E_R_F
Switched to English!
FPGA upgrade succeeded,
Failed to open file!
Fan Speed(%)
Home first,then move z to bottom!
Homing,please wait...
V0.0.11
```

The firmware we have extracted is the real deal

You can find it in [`firmware/mcu.bin`](../firmware/).

Next part, we'll look into deassembling it and understand a bit more how this
embedded system is organized.
