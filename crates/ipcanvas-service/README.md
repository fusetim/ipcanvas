# ipcanvas-service

The `ipcanvas-service` application is the operation center of [`ipcanvas`](https://github.com/fusetim/ipcanvas).

It provides the following functionalities:
- [ ] Receive and validate the Ping event from the `ipcanvas-ping` application.
- [ ] Persist the Canvas state in memory (and on disk regularly).
- [ ] Serve the canvas state to connected clients via WebRTC.
- [ ] Create diff patches for efficient state updates.

## Building and Running

Currently, you can simply build it from the workspace root:

```bash
cargo build -p ipcanvas-service
cargo run -p ipcanvas-service
```

Configuration files, or environment variables, will probably be introduced in the future to customize the service behavior.

