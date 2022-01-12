#!/usr/bin/env python3
content = bytearray(open('font2.bin', 'rb').read())
for i in range(0, len(content), 4):
    content[i], content[i+1] = content[i+1], content[i]

with open('font2a.bin', 'wb') as output:
    output.write(content)
