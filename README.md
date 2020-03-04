# Kekbit
[![Cargo](https://img.shields.io/crates/v/kekbit.svg?color=blue)](
https://crates.io/crates/kekbit)
[![Documentation](https://docs.rs/kekbit/badge.svg)](https://docs.rs/kekbit)
[![Rust 1.31+](https://img.shields.io/badge/rust-1.31+-important.svg)](
https://www.rust-lang.org)
[![GitHub](https://img.shields.io/github/license/motoras/kekbit?color=important)](https://github.com/motoras/kekbit/blob/master/LICENSE)
[![Build](https://github.com/motoras/kekbit/workflows/Build/badge.svg)](https://github.com/motoras/kekbit/actions?query=workflow%3ABuild)
[![Clippy](https://github.com/motoras/kekbit/workflows/Clippy/badge.svg)](https://github.com/motoras/kekbit/actions?query=workflow%3AClippy)
[![codecov](https://codecov.io/gh/motoras/kekbit/branch/master/graph/badge.svg)](https://codecov.io/gh/motoras/kekbit)

Mean and lean composable components for working with ultralight **persistent data channels** in rust. Channels could be used for communication, transaction journaling, live replication of an application state or as a backend for persisting software system images.

## Basic Concepts

#### Persistent data channels
* A mechanism to sequentially persist data at a very fast rate
* They are **writer bound** - it is a writer which creates them and specify the particular structure of a channel such size, maximum record length, or timeout
* They have a fixed predefined capacity. 
* Once a channel is closed, is full, or is abandoned it will never be used again for writing.
* They are byte-oriented sinks
* They are backed by a file; using a RAM disk for storage such as tempfs or /dev/shm could provide blazing fast speeds
* They always use little endian byte order

#### Writers and Readers
* Writers are components which push data into a persistent channel. For each channel there is only one writer.
* Readers are components which poll data from a channel. Data available in the channel could be consumend multiple times, sequential or in paralel by multiple readers.
* The default implementations for both readers and writers are non-blocking
* Readers can also offer a straight `Iterator` API
* Additional features can be plug in by composing together multiple readers or multiple writers

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
kekbit = "0.3.0"
```
See the [Examples](https://github.com/motoras/kekbit/blob/master/examples/README.md) for detailed usage.

## Compatibility

The minimum supported Rust version is 1.31. Any change to this is considered a breaking change.

## License

Licensed under 

 * MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)


#### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any additional terms or conditions.
