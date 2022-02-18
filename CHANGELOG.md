# Changelog

## [0.3.5] 2022-02-18

### Changed

- All dependencies are now uptodate


## [0.3.4] 2021-05-10

### Changed

- Fixed issue number 34 `ShmWriter allows sending non-Send type across threads`

## [0.3.3] 2020-03-25

### Added

- Non blocking retry iterator
- Non blocking multithread writer.
- ReadResult struct
- Conversion method from ShmReader to TimeoutReader
- Conversion method form TryIter to the new RetryIter

### Changed

- Iterators return type is now `ReadResult`.
- Iterators are Fused

### Removed

- shm_timeout_reader, just use instead the into method from ShmReader

## [0.3.2] 2020-03-08

### Added

- TimeoutReader, decorates other readers while checks for writer timeout

### Removed 

- ShmReader timeout checks
- Heartbeat method from Writer
- Special heartbeat handling in reader

## [0.3.1] 2020-03-05

### Added

- Handlers API
- EncoderHandler, SequenceHandler and TimestampHandler
- ChainedHandler for chaining multiple handlers

### Changed

- ShmWriter required a Handler type parameter
- ShmWriter uses a handler to write a record into a channel

## [0.3.0] 2020-02-24

### Added

- Non-blocking Iterator access for ShmReader
- The `exhausted` method in ShmReader which tell us if a channel still provide records or not.
- Major refactoring of the various modules

### Changed

- Header struct was renamed Metadata
- Error handling in ShmWriter
- Encodable trait returns Result
- The read method in Reader was renamed try_read, requires no callback handler, and returns an Option wrapped in a Result

### Removed

- The codecs subcrate
- The core subcrate
- DataFormat structure

## [0.2.3] 2020-02-18

### Changed

- Minor fixes in crates related metadata

## [0.2.2] 2020-02-18

### Added

- Create the new codecs subcrate
- First iteration for the DataFormat and Encodable traits
- RawBinDataFormat for opaque binary data
- PlainTextDataFormat for text data

### Changed

- Writer has now a DataFormat type parameter
- The *write* method from writer requires an Encodable parameter, and got rid of the byte slice and len parameters
- Chat example was fixed, and is now ready to be published


### Fixed

- ShmWriter will try to write one more record than space available

## [0.2.1] 2020-02-14

### Added

- Function *try_shm_reader* a convenient method to create a reader while waiting for a channel to be available.  
- Position parameter to the reader callback. This change will break code written with versions <= 0.1.1
- ShmReader's *total_read* method was renamed *position*
- Method move_to for the Reader traits, so it can resume work from a previous session, or skip records

## [0.1.1] 2020-02-11

### Added

- Initial definition of the Writer and Reader traits
- Implementations of the Writer and Reader traits with a backend based on memory mapped files
- Initial definition of the kekbit channel metadata
- Channel metadata validation
- The echo sample
- The request/reply IPC sample
- README and CHANGELOG documentation
- The child_ps example used for benchmarking with hyperfine