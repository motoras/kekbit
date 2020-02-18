# Kekbit Examples

The following examples provide a quick introduction on how to use kekbit channels.

## Echo
 
This sample illustrates the basic channel operations. A [Writer](https://github.com/motoras/kekbit/blob/master/kekbit-core/examples/echo_in.rs) creates a channel than writes into it every line of text read from the console. A [Reader](https://github.com/motoras/kekbit/blob/master/kekbit-core/examples/echo_out.rs) connects to an existing channel than prints to the console every messages it reads from that channel.
The channel uses the ubicuos [plain text encoder](https://github.com/motoras/kekbit/blob/master/kekbit-codecs/src/codecs/text.rs)
 
 The *Bye* message will stop both the reader and the writer. The channel has a limit of 1000 messages of 1024 bytes each and a timeout of 30 seconds. 
 
In order to start the *writer*, in the kekbit_core folder type:
 ```cargo run --example echo_in <channel_id>```
 
 E.g:
 ```
 cargo run --example echo_in 4242
 ```
 
After the writer had started, in a separate console start the *reader*, from the same kekbit_core folder:
 ```cargo run --example echo_out <channel_id>```
 
 E.g:
 ```
 cargo run --example echo_out 4242
 ```

Be sure you use the same the same *channel_id* for both programs. This example will create channels under `/{tmpf}/kekbit/echo_sample/` folder. Particulary for a channel with the id 4242 in linux, the file `/tmp/kekbit/echo_sample/0000_0000/0000_1092.kekbit` will be created and used as a persistent storage. To avoid unspecified behaviour before reruning the sample, you should either delete the file or run the sample with a different channel id.

## Request/Reply IPC

This sample illustrates a simple request/reply IPC model using kekbit channels. The [Requester](https://github.com/motoras/kekbit/blob/master/kekbit-core/examples/req.rs) creates a channel(the *requests channel*), writes requests to it than reads the replies from another channel(the *replies channel*). The [Replier](https://github.com/motoras/kekbit/blob/master/kekbit-core/examples/rep.rs) creates a channel for the replies(the *replies channel*), reads the requests from the *requests channel*, process them, and writes the replies back on the *replies channel*.

The channel uses the [raw binary encoder](https://github.com/motoras/kekbit/blob/master/kekbit-codecs/src/codecs/raw.rs) but wills switch to a custom encoder in a future iteration.

In order to run the sample, start the requester and the repliers in separate consoles with the following commands:

     For the requester: cargo run --example req <request_channel_id> <reply_channel_id>

     E.g.: cargo run --example req 88 99

     For the replier: cargo run --example rep <reply_channel_id> <request_channel_id>

     E.g.: cargo run --example rep 99 88


To avoid unspecified behaviour before reruning the sample, you should either delete the files associated with these channels(e.g ```rm -rf /tmp/kekbit/req_rep```) or simply use different channel ids.


## Chat
This is a more complete version of the echo application. Two users will start each one an instance of the [chat application](https://github.com/motoras/kekbit/blob/master/kekbit-core/examples/chat.rs), and will be able to exchange messages by typing them into the console. In a more complex scenario you could have a chain of users talking, each reading from a channel and writing in a different one. 
The *Bye* message will stop the conversation.

In order to start a caht instance, in the kekbit_core folder type:
 ```cargo run --example chat <channel_id_1> <channel_id_2>```
 
 E.g:
 ```
 cargo run --example echo_in 4242 4243
 ```

 Than in another console type:
```cargo run --example chat <channel_id_2> <channel_id_1>```
 
 E.g:
 ```
 cargo run --example echo_in 4243 4242
 ```

 The first channel is the one we are writing to, the second is the one we listen too.
 After you start both instances any console input from one will be printed on the other one.

To avoid unspecified behaviour before reruning the sample, you should either delete the files associated with these channels(e.g ```rm -rf /tmp/kekbit/kekchat```) or simply use different channel ids.
