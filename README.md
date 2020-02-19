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



A set of mean, lean and composable components for working with ultralight **persistent data channels** in rust. Such channels could be used for communication, transaction journaling, live replication of an application state or as a backend for persisting software system images.

## Basic Concepts

#### Persistent data channels
* Are a mechanism to sequentially persist data at a very fast rate
* They are **writer bound** - it is a writer which creates them and specify the particular structure of a channel such size, maximum record length, or timeout
* They have a fixed predefined capacity which cannot be changed. 
* Once a channel is closed, full, or abandoned it will never be used again for writing.
* They are backed by a file; using a filesystem with RAM for the backing store such as tempfs or /dev/shm could provide blazing fast speeds
* They always use little endian byte order

#### Writers and Readers
* Writers are components which push data into a persistent channel. For each channel there is only one writer.
* Readers are components which poll data from a channel. Multiple readers could read at any given time from a channel, at their own pace, so the data available in the channel could be consumend multiple times, and in paralel by various readers.

#### Codecs
* Dara could be stored in any format
* [Raw bytes](https://docs.rs/kekbit/*/kekbit/codecs/struct.RawBinDataFormat.html) and [plain text](https://docs.rs/kekbit/*/kekbit/codecs/struct.PlainTextDataFormat.htmll) are the default formats provided
* More complex(JSON, RON) serde-based ones are in development
* Appplication specific data formats could be plugged in 


## Components
The main kekbit crate just re-exports components from its subcrates:

* [`kekbit-core`](kekbit-core)  defines the [`Writer`](https://docs.rs/kekbit/*/kekbit/core/trait.Writer.html) and [`Reader`](https://docs.rs/kekbit/*/kekbit/core/trait.Reader.html) traits together with the [`ShmWriter`](https://docs.rs/kekbit/*/kekbit/core/struct.ShmWriter.html) and [`ShmReader`](https://docs.rs/kekbit/*/kekbit/core/struct.ShmReader.html) implementations which provide write and read operations for memory mapped channels.
 
* [`kekbit-codecs`](kekbit-codecs)  defines the [`DataFormat`](https://docs.rs/kekbit/*/kekbit/codecs/trait.DataFormat.html) and [`Encodable`](https://docs.rs/kekbit/*/kekbit/codecs/trait.Encodable.html) traits used to encode/decode data from channels.


## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
kekbit = "0.2.2"
```
See the [Examples](https://github.com/motoras/kekbit/blob/master/kekbit-core/examples/README.md) for detailed usage.

## Compatibility

The minimum supported Rust version is 1.31. Any change to this is considered a breaking change.

## License

Licensed under 

 * MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)


#### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any additional terms or conditions.

