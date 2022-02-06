.PHONY: build run attach start_opencd start_jlink start_jlink_rtt

build:
	cargo objdump --release -- -h | ./rom_stats.py

run: build
	cargo run --release

attach:
	arm-none-eabi-gdb -q -x gdb/main.gdb target/thumbv7em-none-eabihf/release/app

start_openocd:
	openocd -f jlink.cfg -f target/stm32f1x.cfg

start_jlink:
	JLinkGDBServer -AutoConnect 1 -Device GD32F307VE -If SWD -Speed 4000 -nogui

start_jlink_rtt:
	JLinkRTTClient
