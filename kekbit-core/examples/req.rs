//! Request/Reply IPC sample. This component will write requests to a channel,
//! than  wait for the matching replies on a separate channel. The requests are 3 u64 values:
//! a request id, and 2 values which the repliers is suppose to sum them up.
//! In order to start the requester type cargo run --example req <request_channel_id> <reply_channel_id>
use crossbeam::utils::Backoff;
use kekbit_core::api::Reader;
use kekbit_core::api::Writer;
use kekbit_core::header::Header;
use kekbit_core::shm::shm_reader;
use kekbit_core::shm::shm_writer;
use kekbit_core::tick::TickUnit::Secs;
use std::collections::HashSet;

#[inline]
fn read_u64(data: &[u8], offset: usize) -> u64 {
    assert!(offset + 8 <= data.len());
    u64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ])
}

fn main() {
    let args: Vec<u64> = std::env::args().skip(1).map(|id| id.parse().unwrap()).collect();
    assert!(args.len() == 2);
    let req_id = 0xCDEF;
    let req_channel_id = args[0];
    let reply_channel_id = args[1];
    let timeout_secs = 10; //channel times out in 10 secs
    let tmp_dir = std::env::temp_dir().join("kekbit").join("req_rep");
    let max_msg_size = 1024;
    let header = Header::new(req_id, req_channel_id, max_msg_size * 1000, max_msg_size, timeout_secs, Secs);
    //creates the channel where the requests will be sent together with the associated writer
    let mut writer = shm_writer(&tmp_dir, &header).unwrap();
    let mut tries = 1;
    //tries to connect to the channel from where the replies will be read
    let mut reader_rep = shm_reader(&tmp_dir, reply_channel_id);
    while reader_rep.is_err() && tries <= 100 {
        tries += 1;
        reader_rep = shm_reader(&tmp_dir, reply_channel_id);
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    if reader_rep.is_err() {
        println!("Could not connect to replier. Giving up..");
        std::process::exit(1);
    }
    let backoff = Backoff::new();
    let mut reader = reader_rep.unwrap();
    let mut waiting_for: HashSet<u64> = HashSet::new();
    let requests: Vec<(u64, u64)> = vec![(1, 1), (2, 2), (3, 3), (4, 4), (5, 5)];
    for (i, el) in requests.iter().enumerate() {
        //send a request
        let idx = i as u64;
        let mut msg: [u8; 24] = [0; 24];
        msg[0..8].clone_from_slice(&idx.to_le_bytes());
        msg[8..16].clone_from_slice(&(&el.0).to_le_bytes());
        msg[16..24].clone_from_slice(&(&el.1).to_le_bytes());
        writer.write(&msg, msg.len() as u32).unwrap();
        println!("Sent request {} ", i);
        waiting_for.insert(idx);
        backoff.snooze();
        //check for a reply, it may or may not have come yet
        reader
            .read(
                &mut |bytes_msg| {
                    let id = read_u64(&bytes_msg, 0);
                    let res = read_u64(&bytes_msg, 8);
                    waiting_for.remove(&id);
                    println!("Reply for request {} is {}", id, res);
                },
                1,
            )
            .unwrap();
    }

    //check for all replies which are missing
    while !waiting_for.is_empty() {
        reader
            .read(
                &mut |bytes_msg| {
                    let id = read_u64(&bytes_msg, 0);
                    let res = read_u64(&bytes_msg, 8);
                    waiting_for.remove(&id);
                    println!("Reply for request {} is {}", id, res);
                },
                1,
            )
            .unwrap();
        backoff.spin();
    }
}
