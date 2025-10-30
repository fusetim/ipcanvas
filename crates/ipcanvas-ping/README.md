# ipcanvas-ping

This crate contains the user-space component of the Ping listener for the [ipcanvas project](https://github.com/fusetim/ipcanvas).  
It is responsible for loading the eBPF program into the kernel, and forwarding the received ping information to the ipcanvas service.

## What does it do?

The main responsibilities of this crate are:
1. Load the eBPF program from the `ipcanvas-ping-ebpf` crate into the kernel
2. Attach the eBPF program to a specified network interface using XDP
3. Ensure IP rate limiting (TODO)
4. Forward the received ping information (source and destination IPv6 addresses) to the ipcanvas service
5. Cleanly unload the eBPF program on exit

## Building and Running

It is preferred to build and run this crate using cargo at the workspace root level, to ensure that all dependencies
are correctly resolved, and that `.cargo/config.toml` settings are applied.

That said, you can build and run this crate directly using cargo commands:

```bash
cargo build -p ipcanvas-ping --release
sudo ./target/release/ipcanvas-ping --iface <network-interface> --prefix <ipv6-prefix>
```
