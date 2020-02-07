//! A basic kekbit channel reader. Reads from a kekbit channel and prints on the screen.
//! Will stop if is timing out or if a 'Bye' message is received.
//! Start it with the following command echo_out <writer_id> <channel_id>

use kekbit_core::api::Reader;
use kekbit_core::shm::shm_reader;

fn main() {
    let args: Vec<u64> = std::env::args().skip(1).map(|id| id.parse().unwrap()).collect();
    assert!(args.len() == 2);
    let writer_id = args[0];
    let channel_id = args[1];
    let tmp_dir = std::env::temp_dir().join("kekbit").join("echo_sample");
    let mut reader = shm_reader(&tmp_dir, writer_id, channel_id).unwrap();
    loop {
        let mut do_break = false;
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
                    //just nothign better to do than sleep until a new message comes
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
            }
            Err(err) => {
                println!("Error occured {:?}. Will stop ", err);
                do_break = true;
            }
        }
        if do_break {
            break;
        }
    }
}