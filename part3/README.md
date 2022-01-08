Dumping the external flash
==========================

There's two things we need to accomplish:
* Send large amount of bytes from the device to the host, as we
  are planning to dump 32MB of flash after all. We'll do this with the
  semihosting features.
* Read the SPI flash. Hopefully we can find some already written code to do
  that.

## Sending files from the device to the host

ARM defines semihosting operations meant for a device to communicate with a
host runnign a debugger. See [What is
semihosting?](https://developer.arm.com/documentation/dui0471/i/semihosting/what-is-semihosting-?lang=en)

In a nutshell, the device invokes a system call (via the `BKPT 0xAB` instruction
on the Cortex-M4). This traps the debugger. The register `r0` contains the
system call number, and `r1` contains a pointer to the system call arguments.

The system calls are the classic `SYS_OPEN`, `SYS_WRITE`, etc. The list of
system call can be found
[here](https://developer.arm.com/documentation/dui0471/i/semihosting/semihosting-operations?lang=en).
We can look at the documentation of
[`SYS_OPEN`](https://developer.arm.com/documentation/dui0471/i/semihosting/sys-open--0x01-?lang=en)
or
[`SYS_WRITE`](https://developer.arm.com/documentation/dui0471/i/semihosting/sys-write--0x05-?lang=en).

> SYS_WRITE (0x05)
> 
> Writes the contents of a buffer to a specified file at the current file position.
> Perform the file operation as a single action whenever possible. For example,
> do not split a write of 16KB into four 4KB chunks unless there is no
> alternative.
> 
> On entry, R1 contains a pointer to a three-word data block:
> * word 1: contains a handle for a file previously opened with SYS_OPEN
> * word 2: points to the memory containing the data to be written
> * word 3: contains the number of bytes to be written from the buffer to the file.
> 
> On exit, R0 contains:
> * 0 if the call is successful
> * the number of bytes that are not written, if there is an error.

This is very similar to a hypervisor call if the device was running in a virtual
machine. Except that here, the host is attached via a debugging probe.

Note that invoking a system call pauses the device execution, and thus is not
appropriate for real-time executions as the invokation will be slow (100's of
millisecs).

Another approach to logging is to use [Instrumentation Trace Macrocell
(ITM)](https://developer.arm.com/documentation/ddi0489/f/instrumentation-trace-macrocell-unit),
which is little more involved to setup compared to semihosting as it turns one
of the device pin as an asynchronous serial port, so we'd have to setup the
clock of the device and properly synchronize the device and the host data rates.
The nice thing about semihosting is the richness of the API via the various
system calls. It's not just for logging.

The `cortex-m-semihostring` crate does not expose any API to create files on the
host, even though it seems that we could use the `SYS_OPEN` system call as
OpenOCD implements it well. See its
[implementation](https://github.com/openocd-org/openocd/blob/aad87180586a43500f8af1cf79255c7293bb258b/src/target/semihosting_common.c#L633).
It also implements all the other system call pretty well.

We can add the feature to the `cortex-m-semihosting` crate to export the
`open()` call. See [my pull
request](https://github.com/rust-embedded/cortex-m/pull/387). Hopefully it gets
merged.

Because I am now using my own local copy of the `cortex-m` repository, I add
the following in the `Cargo.toml` file:

```
[patch.crates-io]
cortex-m-semihosting = { path = '../cortex-m/cortex-m-semihosting' }
cortex-m = { path = '../cortex-m' }
```

This instructs cargo to use my version of `cortex-m`, even for dependencies that use
`cortex-m`, like `gd32f3`.

When we run the following code on the device:
```rust
#[entry]
fn main() -> ! {
    use cortex_m_semihosting::{hio::open, nr};
    let mut file = open("hello_world.bin\0", nr::open::RW_TRUNC_BINARY).unwrap();
    file.write_all(b"We can send binaries").unwrap();
    loop {}
}
```

We see the `hello_world.bin` file appear on the host (created by OpenOCD):

```
» hexdump -C hello_world.bin
00000000  57 65 20 63 61 6e 20 73  65 6e 64 20 62 69 6e 61  |We can send bina|
00000010  72 69 65 73                                       |ries|
```

Great! Now we know how to send a file from the device to the host. We are left
with interfacing the external flash chip.

## Sending files from the device to the host
