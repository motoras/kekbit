# Kekbit Examples

The following examples provide a quick introduction on how to use kekbit channels.

## Echo
 
This sample illustrates the basic channel operations. A [Writer](https://github.com/motoras/kekbit/blob/master/kekbit-core/examples/echo_in.rs) creates a channel than writes into it every line of text read from the console. A [Reader](https://github.com/motoras/kekbit/blob/master/kekbit-core/examples/echo_out.rs) connects to an existing channel than prints to the console every messages it reads from that channel.
 
 The *Bye* message will stop both the reader and the writer. The channel has a limit of 1000 messages of 1024 bytes each and a timeout of 30 seconds. 
 
In order to start the *writer*, in the kekbit_core folder type:
 ```cargo run --example echo_in <writer_id>  <channel_id>```
 
 E.g:
 ```
 cargo run --example echo_in 77 4242
 ```
 
After the writer had started, in a separate console start the *reader*, from the same kekbit_core folder:
 ```cargo run --example echo_out <writer_id>  <channel_id>```
 
 E.g:
 ```
 cargo run --example echo_out 77 4242
 ```

Be sure you use the same *writer_id* and the same *channel_id* for both programs. This example will create the file `/tmpfile/kekbit/echo_sample/{writer_id}>/>{channel_id}.kekbit` which will be used as a persistent store for the kekbit channel. To avoid unspecified behaviour before rerun the sample you should either delete the file or run the sample each time with a different channel_id. 

