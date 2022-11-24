# Virtual Serial Port Router (vsp-router)

[![Build status](https://github.com/rfdonnelly/vsp-router/workflows/ci/badge.svg)](https://github.com/rfdonnelly/vsp-router/actions)
[![Crates.io](https://img.shields.io/crates/v/vsp-router.svg)](https://crates.io/crates/vsp-router)

Create virtual serial ports, connect them to physical serial ports, and create routes between them all.

vsp-router was created to connect two terminal emulators to the same physical RS-232 [serial console](https://tldp.org/HOWTO/Remote-Serial-Console-HOWTO/intro-why.html).

[![asciicast](https://asciinema.org/a/519137.svg)](https://asciinema.org/a/519137)

## Supported Operating Systems

* Linux: Yes, tested on Red Hat Enterprise Linux 8
* macOS: Should work but untested
* Windows: No

## Use Cases

Multiplex two virutal serial ports to a single physical serial port.

```sh
vsp-router \
    --virtual 0 \
    --virtual 1 \
    --physical 2:/dev/ttyUSB0 \
    --route 0:2 \
    --route 1:2 \
    --route 2:0 \
    --route 2:1
```

Multiplex two virutal serial ports to a third virtual serial port.

```sh
vsp-router \
    --virtual 0 \
    --virtual 1 \
    --virtual 2 \
    --route 0:2 \
    --route 1:2 \
    --route 2:0 \
    --route 2:1
```

## Example

In terminal A

```sh
cargo run -- \
    --virtual 0 \
    --virtual 1 \
    --virtual 2 \
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

Characters entered in terminal 0 will be sent to terminal 2 only.
Characters entered in terminal 1 will be sent to terminal 2 only.
Characters entered in terminal 2 will be sent to both terminals 0 and 1.

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
    --virtual 0 \
    --virtual 1 \
    --virtual 2 \
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
