# ipcanvas-ping-ebpf

This crate contains the eBPF program that listens for incoming ICMPv6 Echo Requests (pings)
on the delegated IPv6 subnet. It is one part of the Ping listener component of the [ipcanvas project](https://github.com/fusetim/ipcanvas).

## What does it do?

The eBPF program is loaded into the kernel using XDP (eXpress Data Path) to efficiently capture
and process incoming packets at a low level.

It is specifically applied to one network interface that is configured to receive traffic for the
delegated IPv6 subnet.

For every incoming packet, the eBPF program checks: 
1. If the packet is an ICMPv6 packets
2. If it is an Echo Request (ping)
3. If the destination IPv6 address belongs to the delegated subnet.

If all these conditions are met, the program appends both the source IPv6 address and the
destination IPv6 address to a BPF map, which can then be read by a user-space application.
This allows the user-space application to decode the pixel information from the destination
address and update the canvas accordingly.

## Building and Running

You are not really supposed to build or run this crate directly. Instead, it is built and loaded by the
`ipcanvas-ping` crate, which contains the user-space component of the Ping listener.