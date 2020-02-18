# Kekbit-Codecs
An utility subcrate that provides the abstraction required for encoding and decoding channel records in different data formats.
In particular provides the [DataFormat, and Encoder](https://github.com/motoras/kekbit/blob/master/kekbit-codecs/src/codecs.rs) traits which
will be implemented by various data format providers. Besides the data formats already included, user could implement their custom higf performance
data formats  using this traits.

## Data formats provided

### [Raw Binary](https://github.com/motoras/kekbit/blob/master/kekbit-codecs/src/codecs/raw.rs)
    A data format which simply writes raw bytes into the channel whithout any regard of the underlying data's structure
	
### [Plain Text](https://github.com/motoras/kekbit/blob/master/kekbit-codecs/src/codecs/text.rs)
   An unstructured text format. Applications which just want to exchange plain text(such as chat clients or a text file transmission protocol) may
 use this format. It is also a good format for testing.

### Serde based
  Some more complex data formats will be added soon based on the serde library. 

## Usage

This crate can be use directly by adding this to your `Cargo.toml`:
```toml
[dependencies]
kekbit_codecs = "0.1.0"
```
However a better approach will be to use this crate indirectly by adding a dependency to the main kekbit crate.

## Compatibility

The minimum supported Rust version is 1.31. Any change to this is considered a breaking change.

## License

Licensed under 

 * MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)


#### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any additional terms or conditions.

