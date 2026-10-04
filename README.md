# mac-time-server

An [RFC 868](https://www.rfc-editor.org/rfc/rfc868) time server over UDP.

Every datagram received is answered with the current time as a 32-bit
big-endian number of seconds since 1900-01-01 00:00:00 UTC. The contents of
the request are ignored.

## Usage

```
mac-time-server [ADDRESS:PORT]
```

The default address is `0.0.0.0:37`. Port 37 is privileged on most Unix
systems, so either run as root, grant the capability
(`setcap cap_net_bind_service=+ep /usr/bin/mac-time-server`), or listen on a
higher port:

```
mac-time-server 127.0.0.1:3737
```

## Notes

The 32-bit value wraps on 2036-02-07 06:28:16 UTC, as defined by the protocol.
