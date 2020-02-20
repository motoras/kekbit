# Changelog

All notable changes to this project will be documented in this file.


## [0.2.4] In development

### Added
- Decodable trait
- Decodable support for RawBinDataFormat and PlainTextDataFormat
- Non-blocking Iterator access for ShmReader

### Changed
- Error handling in ShmWriter
- Encodable trait returns Result
- The read method in Reader was renamed try_read, requires no callback handler, and returns an Option wrapped in a Result


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
- Implementations of the Writer and Reader traits with a backend  based on memory mapped files
- Initial definition of the kekbit channel metadata
- Channel metadata validation
- The echo sample
- The request/reply IPC sample
- README and CHANGELOG documentation
- The child_ps example used for benchmarking with hyperfine

