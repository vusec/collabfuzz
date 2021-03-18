## Requirements
Requires rust + cargo to build.

Also requires `protoc` to generate protobuffer data structures.

`sudo apt install protobuf`


## Build
Build using `cargo build` or `cargo run`. This will invoke `build.rs` which
generates the protobuf structs for messages.

## How to run
For example:
`RUST_LOG="warn,collab_fuzz::analysis=debug" RUST_BACKTRACE=1 cargo run -- -i /tmp/inputs -o /tmp/collab-out`

## Protocol

Every fuzzer should be assignment an unique ID (control protocol WIP).

Fuzzers should listen to messages with a topic matching its ID, or to messages
with the topic `FUZZER_ID_ALL` (as defined in `src/types.rs`). The
`FUZZER_ID_ALL` topic is used for broadcast jobs.

Jobs are sent to fuzzer opver protobf messages as defined in
`common/seedmsg.proto` (`JobMsg`, which consists of multiple seeds).

Fuzzers communicate with the framework by sending messages to the socket
defined by the env variable `URI_LISTENER` (or `ENV_CONTROL` for control messages). Each message consists of two parts:
1) The message type (`S` for seed updates, and `C` for control messages)
2) The message content appropriate for that message type (a SeedMsg, or
FuzzerCtrlMsg).

See `drivers/example-python/` for an up-to-date example of how to communicate
with the manager.


