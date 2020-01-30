# KeKBiT
[![Build Status](https://travis-ci.org/motoras/kekbit.svg?branch=master)](https://travis-ci.org/motoras/kekbit)
![GitHub](https://img.shields.io/github/license/motoras/kekbit?style=flat-square)
[![Rust 1.31+](https://img.shields.io/badge/rust-1.31+-informational.svg)](
https://www.rust-lang.org)


A set of simple, mean and lean components for working with ultralight persistent data channels in rust. Channels could be used for communication, journaling, application state replication, or for data prevalence systems.

#### Persistent data channel
* Are a mecahnism to sequentially persist data very fast
* They are **writer bound** - it is a writer which creates them and specify the particular structure of a channel such size, maximum record lenght, or writer timeout
* Have a fixed size which cannot be changed. 
* Once a channel is closed, full or abandoned it will never be used again for writing. 
* They are always backed by a file, using a storage filesystem with RAM for the backing store such as tempfs or /dev/shm could provide blazing fast speeds

#### Readers and  Writers

* [`Writer`], a component which writes data into a persitent channel. To each channel only one writer should be assigned. 
* [`Reader`], the core component which reads from a channel. Multiple readers could read at a given time from a channel at their own pace. This allows data channel data to be consumend multiple times, or and in paralel by different  readers.


## Crates
The main kekbit crate just re-exports components from its subcrates:
* [`kekbit-core`](kekbit-core) defines theprovides the main queue components. 

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
kekbit = "0.1.0"
```

## Compatibility

The minimum supported Rust version is 1.31. Any change to this is considered a breaking change.

## License

Licensed under 

 * MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)


#### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any additional terms or conditions.

