# Virtual Serial Port Router (vsp-router)

[![Build status](https://github.com/rfdonnelly/vsp-router/workflows/ci/badge.svg)](https://github.com/rfdonnelly/vsp-router/actions)
[![Crates.io](https://img.shields.io/crates/v/vsp-router.svg)](https://crates.io/crates/vsp-router)

Create virtual serial ports, connect them to physical serial ports, and create routes between them all.

Vsp-router was created to connect two terminal emulators to the same physical RS-232 [serial console](https://tldp.org/HOWTO/Remote-Serial-Console-HOWTO/intro-why.html).

[![asciicast](https://asciinema.org/a/519137.svg)](https://asciinema.org/a/519137)

## Supported Operating Systems

* Linux: Yes, tested on Red Hat Enterprise Linux 8
* macOS: Yes, tested on macOS Ventura 13.1
* Windows: Yes*, tested on Windows 10

*The Windows version does not support creation of virtual serial ports.
A third-party tool like [com0com](https://com0com.sourceforge.net) can be used instead.

## Use Cases

Multiplex two virtual serial ports to a single physical serial port.

```sh
vsp-router \
    --create 0 \
    --create 1 \
    --attach 2:/dev/ttyUSB0 \
    --route 0:2 \
    --route 1:2 \
    --route 2:0 \
    --route 2:1
```

Multiplex two virtual serial ports to a third virtual serial port.

```sh
vsp-router \
    --create 0 \
    --create 1 \
    --create 2 \
    --route 0:2 \
    --route 1:2 \
    --route 2:0 \
    --route 2:1
```

## Example

In terminal A

```sh
cargo run -- \
    --create 0 \
    --create 1 \
    --create 2 \
    --route 0:2 \
    --route 1:2 \
    --route 2:0 \
    --route 2:1
```

In terminal 0

```sh
picocom 0
```

In terminal 1

```sh
picocom 1
```

In terminal 2

```sh
picocom 2
```

Characters entered in terminal 0 will be sent to terminal 2 only.\
Characters entered in terminal 1 will be sent to terminal 2 only.\
Characters entered in terminal 2 will be sent to both terminals 0 and 1.

## Connection to a Virtual Serial Port

Virtual serial ports created by vsp-router behave a bit differently from physical serial ports:

* You can connect to them using any baud rate.
  You are not forced to use the same baud rate as the physical serial port you are multiplexing.
* When you don't read from them they accumulate data in a buffer.
  When this buffer becomes full new data will be discarded and a warning message will be shown in the logs.
  Any buffered data will get returned when you next read from them.

To avoid reading stale data accumulated in the buffer when you want to read from the virtual serial port it is recommended to flush its input buffer before you first read from it.
This can be done using `tcflush(fd, TCIFLUSH)` (or equivalent on your platform) on the file descriptor of the virtual serial port.

## Comparison to TTYBUS

vsp-router is similar to [TTYBUS](https://github.com/danielinux/ttybus).

The key differences is in how data is written.
TTYBUS broadcasts data to all ports.
vsp-router routes data to select ports.

The following 3-port configurations are the equivalent.

TTYBUS

```sh
tty_bus -d -s bus
tty_fake -d -s bus 0
tty_fake -d -s bus 1
tty_fake -d -s bus 2
```

vsp-router

```sh
vsp-router \
    --create 0 \
    --create 1 \
    --create 2 \
    --route 0:1 \
    --route 0:2 \
    --route 1:0 \
    --route 1:2 \
    --route 2:0 \
    --route 2:1
```

## Comparison to socat

Socat establishes a bidirectional channel between two end points.
Vsp-router establishes a multi-directional channel between N end points.
Socat supports several different types of end points.
Vsp-router supports two: virtual serial ports and physical serial ports.

Vsp-router could be combined with socat to enable interesting use cases.
For example, vsp-router could be used to snoop a physical serial port and paired with socat to send the snooped data over UDP.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
