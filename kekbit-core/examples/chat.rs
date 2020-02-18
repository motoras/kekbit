use kekbit_codecs::codecs::text::PlainTextDataFormat;
use kekbit_core::api::{Reader, Writer};
use kekbit_core::header::Header;
use kekbit_core::shm::{shm_writer, try_shm_reader};
use kekbit_core::tick::TickUnit;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

const FOREVER: u64 = 999_999_999_999;

fn run_writer(channel_id: u64, run: Arc<AtomicBool>) {
    let tmp_dir = std::env::temp_dir().join("kekchat");
    let msg_size = 1000;
    let header = Header::new(1111, channel_id, msg_size * 1000, msg_size, FOREVER, TickUnit::Nanos);
    let mut writer = shm_writer(&tmp_dir, &header, PlainTextDataFormat).unwrap();
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
    let mut reader = reader_res.unwrap();
    while run.load(Ordering::Relaxed) == true {
        let mut stop = false;
        let read_res = reader.read(
            &mut |_pos, msg: &[u8]| {
                let msg_str = std::str::from_utf8(&msg).unwrap();
                println!(">>>{}", msg_str);
                if msg_str == "Bye".to_string() {
                    println!("Received Bye. Exiting.....");
                    stop = true;
                }
            },
            10,
        );
        if stop {
            run.store(false, Ordering::Relaxed);
            std::process::exit(0);
        } else {
            match read_res {
                Ok(bytes_count) => {
                    if bytes_count == 0 {
                        std::thread::sleep(Duration::from_millis(300));
                    }
                }
                Err(err) => {
                    println!("Error occured {:?} ", err);
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
    let your_channel_id = args[0];
    let other_channel_id = args[1];
    let run = Arc::new(AtomicBool::new(true));
    let run_w = run.clone();
    let run_r = run.clone();
    let handle_w = std::thread::spawn(move || run_writer(your_channel_id, run_w));
    let handle_r = std::thread::spawn(move || run_reader(other_channel_id, run_r));
    handle_r.join().unwrap();
    handle_w.join().unwrap();
}
