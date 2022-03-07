[![Discord](https://img.shields.io/discord/940395991016828980?label=Discord&logo=discord&logoColor=white)](https://discord.gg/9HSMNYxPAM)

Reverse engineering the Anycubic Photon Mono 4K
===============================================

This documents the reversing of the Mono 4K, which helps in writing the
open-source firmware [Turbo Resin](https://github.com/nviennot/turbo-resin).

## Table of content

* [Part 1: Discovery of the hardware and firmware extraction](/writeup/part1/README.md)
* [Part 2: Planning the read of the external flash](/writeup/part2/README.md)
* [Part 3: Creating a Rust development environment](/writeup/part3/README.md)
* [Part 4: Dumping content of the external flash](/writeup/part4/README.md)
* [Part 5: Graphics extraction from the external ROM](/writeup/part5/README.md)
* [Part 6: Taking control of the user-facing LCD display](/writeup/part6/README.md)
* [Part 7: Detecting touches on the display and improving on the original firmware](/writeup/part7/README.md)
* [Part 8: Driving the Z-axis stepper motor](/writeup/part8/README.md)
* [Part 9: Driving the LCD Panel, and displaying a print layer from the USB stick](/writeup/part9/README.md)

## Introduction

The [Anycubic Mono 4K](https://www.anycubic.com/collections/3d-printers/products/photon-mono-4k)
is my first 3D printer. It's been quite a steep learning curve to print with
resin, but it's really satisfying to make objects once the workflow is ironed
out.

Here's the printer.

![Anycubic Mono 4K](/writeup/part1/printer.jpg)

Here's an example of a printed object.

![Printed Bracket](/writeup/part1/bracket_print.jpg)

~~There are visible lines due to the lack of anti-aliasing support on the printer.
Its LCD screen outputs only fully transparent, or fully opaque pixels, no gray-scale.
Apparently, we have to wait for a firmware update, but I'd rather not wait.~~

Update: The original firmware handles anti-aliasing just fine.

Reverse engineering the printer will guide our implementation of a new firmware,
[Turbo Resin](https://github.com/nviennot/turbo-resin), so we can add the
features that we want.

## Resources

* Datasheets: [datasheet](/datasheet) folder
* Original firmware: [firmware](/firmware) folder
* UI images: [/firmware/ui](/firmware/ui) folder
* PCB photos: [pcb](/pcb) folder
* Pin config: [print_ports_config.py](/firmware/print_ports_config.py) and [port_config.txt](/firmware/port_config.txt)
* Turbo Resin firmware: (https://github.com/nviennot/turbo-resin)
* Our discord channel: https://discord.gg/9HSMNYxPAM

