//! A basic kekbit channel reader. Reads from a kekbit channel and prints on the screen.
//! Will stop if is timing out or if a 'Bye' message is received.
//! Start it with the following command echo_out <channel_id>
use kekbit_core::shm::try_shm_reader;

fn main() {
    let args: Vec<u64> = std::env::args().skip(1).map(|id| id.parse().unwrap()).collect();
    assert!(args.len() == 1);
    let channel_id = args[0];
    let tmp_dir = std::env::temp_dir().join("kekbit").join("echo_sample");
    //try 3 times per second for 20 seconds to conenct to the channel
    let mut reader = try_shm_reader(&tmp_dir, channel_id, 20_000, 60).unwrap();
    let mut stop = false;
    while !stop {
        let mut msg_iter = reader.try_iter();
        for msg in &mut msg_iter {
            let msg_str = std::str::from_utf8(&msg).unwrap();
            println!("Echoing... {}", msg_str);
        }
        if msg_iter.size_hint().1 == Some(0) {
            println!("Nothing more to read. Will stop");
            stop = true;
        } else {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
}
