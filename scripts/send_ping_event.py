#!/usr/bin/env python
#-*- coding: utf-8 -*-

# This script sends ping events to the ipcanvas-service.

import socket
import time

HOST = '127.0.0.1' # The server's hostname or IP address
PORT = 7894        # The port used by the server
SRC_ADDR = "2001:0db8:85a3:0000:0000:8a2e:0370:7334"

def create_ping_event(src_addr, dst_addr):
    # Create a simple ping event packet
    event = bytearray()
    event.extend(socket.inet_pton(socket.AF_INET6, src_addr))  # Source address
    event.extend(socket.inet_pton(socket.AF_INET6, dst_addr))  # Destination address
    return event

def create_pixel_ping(x,y,r,g,b):
    DST_ADDR = f"2001:0db8:85a3:{x:04x}:{y:04x}:00{r:02x}:00{g:02x}:00{b:02x}"
    return create_ping_event(SRC_ADDR, DST_ADDR)

if __name__ == "__main__":
    # Create a TCP socket
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.connect((HOST, PORT))
        redx10y20 = create_pixel_ping(10,20,255,0,0)
        yellowx15y25 = create_pixel_ping(15,25,255,255,0)
        whitex30y40 = create_pixel_ping(30,40,255,255,255)
        greenx256y256 = create_pixel_ping(256,256,0,255,0)
        s.sendall(redx10y20)
        # wait 1s between sends
        s.sendall(yellowx15y25)
        s.sendall(whitex30y40)
        s.sendall(greenx256y256)
        time.sleep(1)
        print('Ping events sent')
        s.close()