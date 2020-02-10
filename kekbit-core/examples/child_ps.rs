use kekbit_core::api::{ReadError, Reader, Writer};
use kekbit_core::header::Header;
use kekbit_core::shm::{shm_reader, shm_writer, storage_path};
use kekbit_core::tick::TickUnit;
use std::process::exit;

use log::{error, info};
use nix::sys::wait::waitpid;
use nix::unistd::{fork, getpid, ForkResult};
use std::path::Path;
use std::result::Result;

const ITERATIONS: u32 = 1 * 1_000_000_0;
const Q_PATH: &str = "/dev/shm";
//const Q_PATH: &str = "./shm/keki";

pub fn run_writer() -> Result<(), ()> {
    info!("Creating writer process ...{}", getpid());
    let chunk_size = 100;
    let header = Header::new(100, 1000, chunk_size * (ITERATIONS + 100), 1000, 99999999999, TickUnit::Nanos);
    let mut writer = shm_writer(&Path::new(Q_PATH), &header).unwrap();
    let msg_bytes = "There are 10 kinds of people: those who know binary and those who don't".as_bytes();
    // let msgs: Vec<&str> = "There are 10 kinds of people: those who know binary and those who don't"
    //     .split_whitespace()
    //     .collect();
    let mut total = 16;
    for _i in 0..ITERATIONS {
        // for m in &msgs {
        //     let to_wr = m.as_bytes();
        //     let len = to_wr.len() as u32;
        //     let res = writer.write(&to_wr, len);
        //     match res {
        //         WriteResult::Success(_) => (),
        //         err => {
        //             error!("Write failed {:?}", err);
        //             panic!("Write failed");
        //         }
        //     }
        // }
        let len = msg_bytes.len() as u32;
        let res = writer.write(&msg_bytes, len);
        match res {
            Ok(b) => {
                total += b;
            }
            Err(err) => {
                error!("Write failed {:?}", err);
                panic!("Write failed");
            }
        };
    }
    info!("We wrote {} bytes ", total);
    Ok(())
}

pub fn run_reader() -> Result<(), ()> {
    info!("Creating reader porcess ...{}", getpid());
    let mut reader = shm_reader(&Path::new(Q_PATH), 1000).unwrap();
    let mut total_bytes = 0u64;
    let mut stop = false;
    let mut msg_count = 0;
    while !stop {
        match reader.read(&mut |_| msg_count += 1, 30u16) {
            Ok(bytes_read) => total_bytes += bytes_read as u64,
            Err(read_err) => match read_err {
                ReadError::Timeout { .. } => {
                    info!("Timeout detected by reader");
                    stop = true;
                }
                ReadError::Closed { bytes_read } => {
                    total_bytes += bytes_read as u64;
                    info!("Closed channel detected by reader");
                    stop = true;
                }
                ReadError::ChannelFull { bytes_read } | ReadError::Failed { bytes_read } => {
                    total_bytes += bytes_read as u64;
                    error!(
                        "Read failed. Will stop. So far we read {} bytes in {} messages",
                        total_bytes, msg_count
                    );
                    panic!("Read failed!!!!");
                }
                _ => panic!("Unknown read error"),
            },
        }
    }
    info!("We read {} bytes in {} messages", total_bytes, msg_count);
    Ok(())
}

fn main() {
    simple_logger::init().unwrap();
    info!("Kekbit Driver PID is {}.", getpid());
    let w_pid = match fork() {
        Ok(ForkResult::Child) => {
            exit(match run_writer() {
                Ok(_) => 0,
                Err(err) => {
                    error!("error: {:?}", err);
                    1
                }
            });
        }

        Ok(ForkResult::Parent { child, .. }) => child,

        Err(err) => {
            panic!("[main] writer fork() failed: {}", err);
        }
    };
    let shm_file_path = storage_path(&Path::new(Q_PATH), 1000);
    while !shm_file_path.exists() {}
    let shm_lock_path = shm_file_path.with_extension("lock");
    while shm_lock_path.exists() {}
    info!("Created ??? {}", shm_file_path.exists());
    let mut rpids = Vec::new();
    for _i in 0..1 {
        let r_pid = match fork() {
            Ok(ForkResult::Child) => {
                exit(match run_reader() {
                    Ok(_) => 0,
                    Err(err) => {
                        error!("error: {:?}", err);
                        1
                    }
                });
            }

            Ok(ForkResult::Parent { child, .. }) => child,

            Err(err) => {
                panic!("[main] reader fork() failed: {}", err);
            }
        };
        rpids.push(r_pid);
    }
    for r_pid in rpids {
        match waitpid(r_pid, None) {
            Ok(status) => info!("[main] Reader {} completed with status {:?}", r_pid, status),
            Err(err) => panic!("[main] waitpid() on reader {} failed: {}", r_pid, err),
        }
    }
    match waitpid(w_pid, None) {
        Ok(status) => info!("[main] Writer completed with status {:?}", status),
        Err(err) => panic!("[main] waitpid() on writer failed: {}", err),
    }
    if shm_file_path.exists() {
        std::fs::remove_file(&shm_file_path).unwrap();
        info!("Channel data file {:?} removed", &shm_file_path);
    }
    info!("Kekbit Driver Done!");
}
