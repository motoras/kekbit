# Kekbit-Core
This subcrate defines the main abstractions and provides the core components required to work with kekbit channels. In particular provides
the [ShmReader](https://github.com/motoras/kekbit/blob/master/kekbit-core/src/shm/reader.rs) and [ShmWriter](https://github.com/motoras/kekbit/blob/master/kekbit-core/src/shm/writer.rs) for reading and writing to a memory mapped channel.

## Usage

This crate can be use directly by adding this to your `Cargo.toml`:
```toml
[dependencies]
kekbit_core = "0.1.0"
```

A better approach will be to use this crate indirectly by adding a dependency to the main kekbit crate.

## Compatibility

The minimum supported Rust version is 1.31. Any change to this is considered a breaking change.

## License

Licensed under 

 * MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)


#### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any additional terms or conditions.

