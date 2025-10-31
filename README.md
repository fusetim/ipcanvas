# ipcanvas

The current trend seems to be online canvases where everyone can participate by placing one pixel at
a time to create illustrations on a shared canvas.  
This project is the same concept, except you need to know how to ping an IPv6 address to play.

## ipcanvas as a player

A player ever only see the online canvas, the current status of the grid of pixels, through a website.

To place a pixel, the player needs to send an ICMPv6 Echo Request (ping) to a specific IPv6 address
that encodes the pixel's coordinates and color in the address itself.

Assuming we owned the IPv6 prefix `2001:aaaa:bbbb::/48`, to place a red pixel (RGB: `255,0,0`) at
coordinates `(x=300, y=400)`, the player would send a ping to the address:

```2001:aaaa:bbbb:300:400:255:00:00``` or ```2001:aaaa:bbbb:300:400:ff:00:00``` (if we prefer hex).

The server hosting the canvas would listen for incoming pings, decode the pixel information from
the destination address, and update the canvas accordingly.

## ipcanvas as an adminsys

### Requirements

- An IPv6 subnet allocated to you (e.g., a /48 prefix like `2001:aaaa:bbbb::/48`).
- A server which is delegated this previous IPv6 subnet.
- Additional IPv4/IPv6 addresses to host the canvas service (because the main subnet is used for pixel placement).

### How does it work?

ipcanvas is split into three main components:

1. [**Ping Listener**](./crates/ipcanvas-ping/): A service that listens for incoming ICMPv6 Echo Requests (pings) on the delegated IPv6 subnet.   
  Under the hood, it uses XDP and eBPF to efficiently capture and process the incoming packets at a low level.  
  *Probably overkilled but it is also a nice learning experience!*

2. [**Canvas Service**](./crates/ipcanvas-service/): A very lightweight web services that published the current state of the canvas via WebRTC.

3. **Web Frontend**: A (static?) web application that displays the canvas to users and allows them to see the real-time updates as pixels are placed.

## Development

### Requirements

1. Rust (with Cargo)
2. an Aya-rs ready dev environment for eBPF (see https://aya-rs.dev/book/start/development/)
3. bpf-linker (see item 2)
4. Node.js and (p)npm 

### Building and Running

More details in the subfolder README.

## Copyright - License

This project is licensed under the AGPL-3.0 License. See the [LICENSE file](./LICENSE.md) for details.

Fusetim (2025) - All rights reserved.