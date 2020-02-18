//! A basic kekbit channel writer. Creates  a channel and writes to it whatever it gets from
//! the console. The maximum message size is 1024, the channel size is bound to 1000 such messages.
//! The channel will timeout after 30 seconds of inactivity.
//! Start it with the following command echo_in <channel_id>
use kekbit_codecs::codecs::text::PlainTextDataFormat;
use kekbit_core::api::Writer;
use kekbit_core::header::Header;
use kekbit_core::shm::shm_writer;
use kekbit_core::tick::TickUnit::Secs;

fn main() {
    let args: Vec<u64> = std::env::args().skip(1).map(|id| id.parse().unwrap()).collect();
    assert!(args.len() == 1);
    let timeout_secs = 30; //channel times out in 30 secs
    let writer_id = 7879u64;
    let channel_id = args[0];
    let tmp_dir = std::env::temp_dir().join("kekbit").join("echo_sample");
    let max_msg_size = 1024;
    let header = Header::new(writer_id, channel_id, max_msg_size * 1000, max_msg_size, timeout_secs, Secs);
    let mut writer = shm_writer(&tmp_dir, &header, PlainTextDataFormat).unwrap();
    let mut last_msg_time = Secs.nix_time();
    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Failed read");
        let ts = Secs.nix_time();
        if ts - last_msg_time > timeout_secs {
            println!("Timeout occured. Message will not be sent. Channel will be closed.");
            break;
        } else {
            last_msg_time = ts;
        }
        let data = input.trim();
        writer.write(&data).unwrap();
        if data == "Bye".to_string() {
            println!("Exiting.....");
            break;
        }
    }
}
