Reverse engineering the Anycubic Mono 4K
========================================

## Table of content

* [Part 1: Discovery of the hardware and firmware extraction](/writeup/part1/README.md)
* [Part 2: Planning the read of the external flash](/writeup/part2/README.md)
* [Part 3: Creating a Rust development environment](/writeup/part3/README.md)
* [Part 4: Dumping content of the external flash](/writeup/part4/README.md)
* [Part 5: Graphics extraction from the external ROM](/writeup/part5/README.md)
* [Part 6: Taking control of the user-facing LCD display](/writeup/part6/README.md)

## Introduction

The [Anycubic Mono 4K](https://www.anycubic.com/collections/3d-printers/products/photon-mono-4k)
is my first 3D printer. It's been quite a steep learning curve to print with
resin, but it's really satisfying to make objects once the workflow is ironed
out.

Here's the printer.

![Anycubic Mono 4K](/writeup/part1/printer.jpg)

Here's an example of a printed object.

![Printed Bracket](/writeup/part1/bracket_print.jpg)

There are visible lines due to the lack of anti-aliasing support on the printer.
Its LCD screen outputs only fully transparent, or fully opaque pixels, no gray-scale.
Apparently, we have to wait for a firmware update, but I'd rather not wait.

## Goal

* Replace the firmware of the printer, so we can implement the features we want.
  For example:
* Add anti-aliasing support
* Provide multiple exposure support for a given layer. This would be helpful to print
  exposure calibration objects. A bit like the R_E_R_F feature.
* Optimize print speed. During printing, The lift speed is a big deal. Too slow, and the print takes
  too long and my patience runs down. Too fast, and the print delaminates and
  it's trash. Perhaps we can detect the tensile pressure while lifting, or when
  the printed layer unstick from the FEP film to optimize printing speed
* Add a temperature control unit to keep the resin at temperature
* Accept USB sticks as usual, but also allow the printer to act like a device
  (USB OTG), and be controlled by a host.
* Write the new firmware purely in [Rust](https://www.rust-lang.org/what/embedded)!

## Writeup

* [Part 1: Discovery of the hardware and firmware extraction](/writeup/part1/README.md)
* [Part 2: Planning the read of the external flash](/writeup/part2/README.md)
* [Part 3: Creating a Rust development environment](/writeup/part3/README.md)
* [Part 4: Dumping content of the external flash](/writeup/part4/README.md)
* [Part 5: Graphics extraction from the external ROM](/writeup/part5/README.md)
* [Part 6: Taking control of the user-facing LCD display](/writeup/part6/README.md)


## Resources

* Datasheets: [datasheet](/datasheet) folder
* Original firmware: [firmware](/firmware) folder
* UI images: [/firmware/ui](/firmware/ui) folder
* PCB photos: [pcb](/pcb) folder
* Pin config: [print_ports_config.py](/firmware/print_ports_config.py) and [port_config.txt](/firmware/port_config.txt)
* Rust firmware: [src](/src) folder

