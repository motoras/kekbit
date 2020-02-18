//! Request/Reply IPC sample. This component will read requests from a channel,
//! than send replies on a separate channel. The requests is expected to be three u64 values:
//! a request id, and 2 values which the replier will add them up. The reply will be two
//! u64 values: the id of the request and the sum of the two values from request.
//! In order to start the replier type cargo run --example rep <reply_channel_id> <request_channel_id>
use crossbeam::utils::Backoff;
use kekbit_codecs::codecs::raw::RawBinDataFormat;
use kekbit_core::api::Reader;
use kekbit_core::api::Writer;
use kekbit_core::header::Header;
use kekbit_core::shm::shm_writer;
use kekbit_core::shm::try_shm_reader;
use kekbit_core::tick::TickUnit::Secs;

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
    let rep_id = 0xFEDC;
    let reply_channel_id = args[0];
    let req_channel_id = args[1];
    let timeout_secs = 10; //channel times out in 10 secs
    let tmp_dir = std::env::temp_dir().join("kekbit").join("req_rep");
    let max_msg_size = 1024;
    let header = Header::new(
        rep_id,
        reply_channel_id,
        max_msg_size * 1000,
        max_msg_size,
        timeout_secs,
        Secs,
    );
    //creates the channel where the replies will be sent together with the associated writer
    let mut writer = shm_writer(&tmp_dir, &header, RawBinDataFormat).unwrap();
    //tries to connect to the channel where the requests are pushed
    let reader_rep = try_shm_reader(&tmp_dir, req_channel_id, 5000, 15);
    if reader_rep.is_err() {
        println!("Could not connect to request channel. Giving up..");
        std::process::exit(1);
    }
    let backoff = Backoff::new();
    let mut reader = reader_rep.unwrap();
    //tries to read the requests
    loop {
        let read_res = reader.read(
            &mut |pos, bytes_msg| {
                //extracts the request info
                let id = read_u64(&bytes_msg, 0);
                println!("Got request {} at pos {}", id, pos);
                let first = read_u64(&bytes_msg, 8);
                let second = read_u64(&bytes_msg, 16);
                //compute and sent the reply
                let res: u64 = first + second;
                let mut reply: [u8; 16] = [0; 16];
                reply[0..8].clone_from_slice(&id.to_le_bytes());
                reply[8..16].clone_from_slice(&res.to_le_bytes());
                writer.write(&reply).unwrap();
                println!("Reply for {} sent", id);
            },
            5,
        );
        match read_res {
            Ok(_) => backoff.snooze(),
            Err(err) => {
                //If any error is returned we gave up.
                //Errors include timeout or reaching the 'Close' marker
                println!("No more requests to read. {:?}", err);
                break;
            }
        }
    }
}
