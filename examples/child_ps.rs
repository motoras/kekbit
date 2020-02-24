use kekbit::api::ReadError::*;
use kekbit::api::{Reader, Writer};
use kekbit::core::*;
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
    let metadata = Metadata::new(100, 1000, chunk_size * (ITERATIONS + 100), 1000, 99999999999, TickUnit::Nanos);
    let mut writer = shm_writer(&Path::new(Q_PATH), &metadata).unwrap();
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
        let res = writer.write(&msg_bytes);
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
    let mut reader = try_shm_reader(&Path::new(Q_PATH), 1000, 2000, 200).unwrap();
    let mut stop = false;
    let mut msg_count = 0;
    while !stop {
        match reader.try_read() {
            Ok(Some(_)) => msg_count += 1,
            Ok(None) => (),
            Err(read_err) => match read_err {
                Timeout(_) => {
                    info!("Timeout detected by reader");
                    stop = true;
                }
                Closed => {
                    info!("Closed channel detected by reader");
                    stop = true;
                }
                ChannelFull | Failed => {
                    error!("Read failed. Will stop. So far we read {} messages", msg_count);
                    panic!("Read failed!!!!");
                }
            },
        }
    }
    info!(
        "We read {} bytes in {} messages. Channel state is {:?}",
        reader.position(),
        msg_count,
        reader.exhausted()
    );
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
    let shm_file_path = storage_path(&Path::new(Q_PATH), 1000);
    if shm_file_path.exists() {
        std::fs::remove_file(&shm_file_path).unwrap();
        info!("Channel data file {:?} removed", &shm_file_path);
    }
    info!("Kekbit Driver Done!");
}
