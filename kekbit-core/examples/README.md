# Kekbit Examples

The following examples provide a quick introduction on how to use kekbit channels.

## Echo
 
This sample illustrates the basic channel operations. A [Writer](https://github.com/motoras/kekbit/blob/master/kekbit-core/examples/echo_in.rs) creates a channel than writes into it every line of text read from the console. A [Reader](https://github.com/motoras/kekbit/blob/master/kekbit-core/examples/echo_out.rs) connects to an existing channel than prints to the console every messages it reads from that channel.
 
 The *Bye* message will stop both the reader and the writer. The channel has a limit of 1000 messages of 1024 bytes each and a timeout of 30 seconds. 
 
In order to start the *writer*, in the kekbit_core folder type:
 ```cargo run --example echo_in <channel_id>```
 
 E.g:
 ```
 cargo run --example echo_in 4242
 ```
 
After the writer had started, in a separate console start the *reader*, from the same kekbit_core folder:
 ```cargo run --example echo_out  <channel_id>```
 
 E.g:
 ```
 cargo run --example echo_out 4242
 ```

Be sure you use the same the same *channel_id* for both programs. This example will create channels under `/{tmpf}/kekbit/echo_sample/` folder. Particulary for a channel with the id 4242 in linux, the file `/tmp/kekbit/echo_sample/0000_0000/0000_1092.kekbit` will be created and used as a persistent storage. To avoid unspecified behaviour before reruning the sample, you should either delete the file or run the sample with a different channel id.

## Request/Reply IPC

This sample illustrates a simple request/reply IPC model using kekbit channels. The [Requester](https://github.com/motoras/kekbit/blob/master/kekbit-core/examples/req.rs) creates a channel(the *requests channel*), writes requests to it than reads the replies from another channel(the *replies channel*). The [Replier](https://github.com/motoras/kekbit/blob/master/kekbit-core/examples/rep.rs) creates a channel for the replies(the *replies channel*), reads the requests from the *requests channel*, process them, and writes the replies back on the *replies channel*.

In order to run the sample, start the requester and the repliers in separate consoles with the following commands:

     For the requester: ```cargo run --example req <request_channel_id> <reply_channel_id>```

     E.g.
        ```cargo run --example req 88 99```

     For the replier: ```cargo run --example rep <reply_channel_id> <request_channel_id>```

     E.g.
        ```cargo run --example rep 99 88```   


To avoid unspecified behaviour before reruning the sample, you should either delete the files associated with these channels(e.g ```rm -rf /tmp/kekbit/req_rep```) or simply use different channel ids.
