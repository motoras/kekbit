//! A basic kekbit channel reader. Reads from a kekbit channel and prints on the screen.
//! Will stop if is timing out or if a 'Bye' message is received.
//! Start it with the following command echo_out <channel_id>
use kekbit::core::try_shm_reader;
use kekbit::core::ReadResult;
use kekbit::core::ShmReader;
use kekbit::retry::RetryIter;
fn main() {
    let args: Vec<u64> = std::env::args().skip(1).map(|id| id.parse().unwrap()).collect();
    assert!(args.len() == 1);
    let channel_id = args[0];
    let tmp_dir = std::env::temp_dir().join("kekbit").join("echo_sample");
    //try 3 times per second for 20 seconds to connect to the channel
    let mut reader = try_shm_reader(&tmp_dir, channel_id, 20_000, 60).unwrap();
    let mut msg_iter: RetryIter<ShmReader> = reader.try_iter().into();
    for read_res in &mut msg_iter {
        match read_res {
            ReadResult::Record(msg) => {
                let msg_str = std::str::from_utf8(&msg).unwrap();
                println!("Echoing... {}", msg_str);
            }
            ReadResult::Nothing => {
                //sleep for a while
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            ReadResult::Failed(err) => {
                println!("Echo channel read error {:?}", err);
                break;
            }
        }
    }
}
