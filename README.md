# KekbiT
[![Build Status](https://travis-ci.org/motoras/kekbit.svg?branch=master)](https://travis-ci.org/motoras/kekbit)
[![Build](https://github.com/motoras/kekbit/workflows/Build/badge.svg)](https://github.com/motoras/kekbit/actions?query=workflow%3ABuild)
![Clippy](https://github.com/motoras/kekbit/workflows/Clippy/badge.svg)(https://github.com/motoras/kekbit/actions?query=workflow%3AClippy)
[![Rust 1.31+](https://img.shields.io/badge/rust-1.31+-informational.svg)](
https://www.rust-lang.org)
![GitHub](https://img.shields.io/github/license/motoras/kekbit?style=flat-square)



A set of mean lean and composable components for working with ultralight **persistent data channels** in rust. Such channels could be used for communication, transaction journaling, live mirroring of an application state or as a backend for persisting software system images.

## Basic Concepts

#### Persistent data channels
* Are a mechanism to sequentially persist data at a very fast rate
* They are **writer bound** - it is a writer which creates them and specify the particular structure of a channel such size, maximum record length, or timeout
* They have a fixed predefined capacity which cannot be changed. 
* Once a channel is closed, full, or abandoned it will never be used again for writing.
* They are backed by a file; using a filesystem with RAM for the backing store such as tempfs or /dev/shm could provide blazing fast speeds

#### Writers and Readers
* Writers are components which push data into a persistent channel. For each channel there is only one writer.
* Readers are components which poll data from a channel. Multiple readers could read at any given time from a channel, at their own pace, so the data available in the channel could be consumend multiple times, and in paralel by various readers.


## Components
The main kekbit crate just re-exports components from its subcrates:
* [`kekbit-core`](kekbit-core)  defines the [`Writer`] and [`Reader`] traits together with the [`ShmWriter`] and [`ShmReader`] implementations which provide write and read operations for memory mapped channels.
 

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
kekbit = "0.1.0"
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

