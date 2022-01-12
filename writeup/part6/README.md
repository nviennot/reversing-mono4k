Part 6: Taking control of the user-facing LCD display
=====================================================

In the [previous part](../part5/README.md), we extracted the UI backgrounds and
other various fonts. Let's try to draw them on the printer's display.

Because there are two LCD screens, the user-facing LCD display, and the resin
masking display, we'll call the user-facing one the TFT display to
differentiate them.

![lcd_connector.jpg](lcd_connector.jpg)

There are 27 signal wires connecting the MCU to the TFT display. With this
number of wires, we are not going to try to guess how to interface with it. We
are going to disassemble the MCU firmware downloaded in [Part
1](../part1/README.df) and try to see how the code uses the display.

A visual inspection of the firmware:

```
convert -size 1024x1745 -depth 1 GRAY:mcu.bin'[0]' mcu.png
```

![MCU ROM](mcu.jpg)

![Font1](font1.png)

![Font2](font2.png)

There's two programs in the MCU:
* **The bootloader**: This is the program that runs immediately on power-on. Its
  role is to flash the MCU during a firmware from the external flash chip.
  The way the firmware gets updated is that it gets copied from a USB dongle to
  the external flash, and then the machine restarts. The bootloader flashes the
  firmware from the external flash, and on success, clears the external flash
  from the firmware. This is so that if there's  a loss of power during a
  firmware update, the machine won't be bricked. We'll expand on into this later,
  it will be important for users flashing our firmware into their machines via
  a USB dongle.
* **The main program**: This contains all the logic to run the 3D printer. The
    bootloader transfers execution once done with flashing the firmware.

## Disassembling the bootloader

Luckily for us, the bootloader shows an update message while it's flashing the
MCU with a new firmware. That means that its initializes the TFT display. We are
going to look at what it's doing.

Some research gives us pointers on how to decompile instructions for an STM32
like microcontroller. See [Reverse engineering of ARM
microcontrollers](https://rdomanski.github.io/Reverse-engineering-of-ARM-Microcontrollers/)
and [Reverse Engineering
Radios](https://do1alx.de/2022/reverse-engineering-radios-arm-binary-images-in-ida-pro/)

Note that I use IDA Pro, but I don't necessarily recommend it. I use because I'm
used to it. [Ghidra](https://ghidra-sre.org) is a very capable
disassembler/decompiler, and I would recommend it to newcomers.

After a bit of work, here's the `main` function of the bootloader:

![bootloader main](bootloader_main.png)

I've annotated the code to see things a little clearer. We can see:
* Initialization of various peripherals: the system clock, the external flash,
  and the TFT display.
* The flashing procedure, executed when the string `"FIRE"` is present the
  external flash at address `0xEF0000`.
* And an execution transfer control to the `MAIN_PROG`

## Intializing the I/O pins to the display, and the external memory controller

We are going to study the TFT display initialization procedure as we are going
to replicate it.

![tft_init_stub](tft_init_stub.png)

First, the I/O pin to interact with the TFT display are initialized. Then, the
EXMC controller. The TFT display exposes its frame buffer through an SDRAM
controller. Our microcontroller has a peripheral that can connect such device,
the External Memory Controller  `EXMC` (on GD32s) a.k.a the Flexible Static
Memory Controller `FSMC` (on STM32s).

I'm grepping through the SDK of the `GD32F30x` (provided by the manufacturer)
to find examples of code using the `EXMC` as it's pretty hairy to initialize.
I land on an LCD display initialization example. The developers of the
Mono 4K used the code example from the SDK. Neat, we understands the
semantics of of the values they are using:

```c
bus_latency = 10;
asyn_data_setuptime = 15;
asyn_address_holdtime = 8;
asyn_address_setuptime = 8;
```

We can start implementing our code. I found the
[`stm32-fmc`](https://github.com/stm32-rs/stm32-fmc) crate that does a good
job at abstracting away the details of `EXMC` register configuration.

## Initializing the TFT display

![tft_init](tft_init.png)
