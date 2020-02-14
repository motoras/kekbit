# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] In Progress

### Added
- Function *try_shm_reader* a convenient method to create a reader while waiting for a channel to be available.  
- Position parameter to the reader callback. This change will break code written with versions <= 0.1.1

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

