//! A chat sample which allows multiple instances to communicate
//! by writing/reading messages from the console.
use kekbit::api::EncoderHandler;
use kekbit::api::Writer;
use kekbit::core::*;
use kekbit::retry::RetryIter;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

const TIMEOUT: u64 = 30;

fn run_writer(channel_id: u64, run: Arc<AtomicBool>) {
    let tmp_dir = std::env::temp_dir().join("kekchat");
    let msg_size = 1000;
    let metadata = Metadata::new(1111, channel_id, msg_size * 1000, msg_size, TIMEOUT, TickUnit::Secs);
    let mut writer = shm_writer(&tmp_dir, &metadata, EncoderHandler::default()).unwrap();
    std::thread::yield_now();
    while run.load(Ordering::Relaxed) == true {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Failed read");
        let data = input.trim();
        if data.len() > 0 {
            writer.write(&data).unwrap();
            if input.trim() == "Bye".to_string() {
                println!("Sent Bye. Exiting.....");
                run.store(false, Ordering::Relaxed);
                break;
            }
        }
    }
}

fn run_reader(channel_id: u64, run: Arc<AtomicBool>) {
    let tmp_dir = std::env::temp_dir().join("kekchat");
    let reader_res = try_shm_reader(&tmp_dir, channel_id, 10_000, 30);
    if reader_res.is_err() {
        println!("Could not connect to chat partner");
        std::process::exit(0);
    }
    let mut reader = shm_timeout_reader(reader_res.unwrap());
    let mut msg_iter: RetryIter<TimeoutReader<ShmReader>> = reader.try_iter().into();
    while run.load(Ordering::Relaxed) == true {
        for read_res in &mut msg_iter {
            match read_res {
                ReadResult::Record(msg) => {
                    let msg_str = std::str::from_utf8(&msg).unwrap();
                    println!(">>>{}", msg_str);
                    if msg_str == "Bye".to_string() {
                        println!("Received Bye. Exiting.....");
                        run.store(false, Ordering::Relaxed);
                        std::process::exit(0);
                    }
                }
                ReadResult::Nothing => {
                    std::thread::sleep(Duration::from_millis(300));
                    break;
                }
                ReadResult::Failed(err) => {
                    println!("Chat channel read error {:?}", err);
                    run.store(false, Ordering::Relaxed);
                    std::process::exit(0);
                }
            }
        }
    }
}

pub fn main() {
    let args: Vec<u64> = std::env::args().skip(1).map(|id| id.parse().unwrap()).collect();
    assert!(args.len() == 2);
    let your_channel_id = args[0]; //channel where you will write messages
    let other_channel_id = args[1]; //channel from where you will read messages
    let run = Arc::new(AtomicBool::new(true));
    let run_w = run.clone();
    let run_r = run.clone();
    let handle_w = std::thread::spawn(move || run_writer(your_channel_id, run_w));
    let handle_r = std::thread::spawn(move || run_reader(other_channel_id, run_r));
    handle_r.join().unwrap();
    handle_w.join().unwrap();
}
