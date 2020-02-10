//! A basic kekbit channel reader. Reads from a kekbit channel and prints on the screen.
//! Will stop if is timing out or if a 'Bye' message is received.
//! Start it with the following command echo_out <writer_id> <channel_id>

use kekbit_core::api::Reader;
use kekbit_core::shm::shm_reader;

fn main() {
    let args: Vec<u64> = std::env::args().skip(1).map(|id| id.parse().unwrap()).collect();
    assert!(args.len() == 1);
    let channel_id = args[0];
    let tmp_dir = std::env::temp_dir().join("kekbit").join("echo_sample");
    let mut reader = shm_reader(&tmp_dir, channel_id).unwrap();
    let mut stop = false;
    while !stop {
        let read_res = reader.read(
            &mut |msg: &[u8]| {
                let msg_str = std::str::from_utf8(&msg).unwrap();
                println!("{}", msg_str);
            },
            10,
        );
        match read_res {
            Ok(bytes_count) => {
                if bytes_count == 0 {
                    //just nothing better to do than sleep until a new message comes
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
            }
            Err(err) => {
                println!("Error occured {:?}. Will stop ", err);
                stop = true;
            }
        }
    }
}
