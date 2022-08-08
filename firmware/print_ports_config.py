#!/usr/bin/env python3

# XXX There's a better tool: https://github.com/nviennot/stm32-port-monitor/
# You can monitor the STM32 ports in real-time with this.
# The tool connects to GDB, reads the port registers, and print their status.

# Mem32 0x40010800, 4
# Mem32 0x40010C00, 4
# Mem32 0x40011000, 4
# Mem32 0x40011400, 4
# Mem32 0x40011800, 4
# Mem32 0x40011C00, 4
# Mem32 0x40012000, 4

# mdw 0x40010800 4
# mdw 0x40010C00 4
# mdw 0x40011000 4
# mdw 0x40011400 4
# mdw 0x40011800 4
# mdw 0x40011C00 4
# mdw 0x40012000 4

tft_config = [
  ('A', 0x43444444, 0x44444444, 0x0000ffff, 0x00000040),
  ('B', 0x44484444, 0x44444444, 0x0000fe2a, 0x00000010),
  ('C', 0x43444444, 0x44444444, 0x0000ffff, 0x00000040),
  ('D', 0xb4bb44bb, 0xbb44bbbb, 0x0000f7ff, 0x00000080),
  ('E', 0xb4444444, 0xbbbbbbbb, 0x0000ff8d, 0x00000000),
  ('F', 0x44444444, 0x44444444, 0x00000d95, 0x00000000),
  ('G', 0x44444444, 0x44444444, 0x0000e48a, 0x00000000),
]
config = tft_config

finished_config = [
  ('A', 0xBBB3B3B3, 0x88844383, 0x0000C615, 0x0000A615),
  ('B', 0x33088440, 0xBBB3734B, 0x0000FE0A, 0x00001C18),
  ('C', 0x33444333, 0x44344483, 0x0000FEFF, 0x000022C7),
  ('D', 0xB0BB00BB, 0xBB43BBBB, 0x0000E7B3, 0x00000080),
  ('E', 0xB3334434, 0xBBBBBBBB, 0x0000FF8D, 0x00000000),
  ('F', 0x44444444, 0x44444444, 0x00000D94, 0x00000000),
  ('G', 0x44444444, 0x44444444, 0x0000E48A, 0x00000000),
]

for (port, ctl0, ctl1, istat, octl) in config:
    ctl = (ctl1 << 32) | ctl0
    for pin in range(16):
        pin_mode = (ctl >> (4*pin)) & 0b11
        pin_ctl = (ctl >> (4*pin+2)) & 0b11
        pin_octl = (octl >> pin) & 0b1
        pin_istat = (octl >> pin) & 0b1

        is_input = pin_mode == 0

        if is_input:
            desc = {
                0b00: 'Analog input',
                0b01: f'Input floating v={pin_istat}',
                0b10: f'Input pull-up v={pin_istat}' if pin_octl else f'input pull-down v={pin_istat}',
                0b11: 'Input: INVALID',
            }[pin_ctl]
        else:
            desc = {
                0b00: f'Output push-pull v={pin_octl}',
                0b01: f'Output open-drain v={pin_octl}',
                0b10: 'Alternate output push-pull',
                0b11: 'Alternate output open-drain',
            }[pin_ctl]

            speed = {
                0b01: '10Mhz',
                0b10: '2Mhz',
                0b11: '50Mhz',
            }[pin_mode]
            desc = f'{desc} speed={speed}'

        print(f"P{port}{pin} {desc}")
