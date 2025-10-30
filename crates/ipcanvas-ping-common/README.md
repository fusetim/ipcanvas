# ipcanvas-ping-common

This crate is a shared library, containing the data structure definitions shared by both the userspace 
and eBPF components of `ipcanvas-ping`.

As such, most of those structs are annotated with `#[repr(C)]` to ensure that their memory layout is compatible
between the two environments, as they are shared through eBPF maps.

Other than that, this crate does not contain any significant logic or functionality on its own.

## Particularities

This crate is compiled twice: once for the eBPF target, and once for the native target.  
When compiled for eBPF, the crate must not depend on any standard library features, as eBPF programs run in a
restricted environment within the kernel. However, when compiled for the native target, the crate can use standard 
library features as needed, which is possible by enabling the optional `user` feature flag during compilation.