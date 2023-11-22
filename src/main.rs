// @author Max Bebök
// @license MIT

mod gpu;
mod checksum;

use std::{env, fs};
use std::io::Read;
use std::time::Instant;
use futures::executor::block_on;
use clap::Parser;
use gpu::compute::GPUCompute;

const BOOTCODE_OFFSET: u32 = 0x40;
const BOOTCODE_SIZE: u32 = 0x1000;
const BOOTCODE_SIZE_WORDS: u32 = BOOTCODE_SIZE / 4;

const GPU_BATCH_COUNT: u32 = 16*8; // keep in sync with shader!
const GPU_GROUP_SIZE_XY: u32 = 512;
//const GPU_BATCH_COUNT: u32 = 2; // DEBUG
//const GPU_GROUP_SPLIT: u32 = 2;
const GPU_STEP_SIZE: u32 = GPU_GROUP_SIZE_XY * GPU_GROUP_SIZE_XY * GPU_BATCH_COUNT;

async fn bruteforce_run(data: Vec<u32>) -> Option<Vec<u32>> {

  let gdu_compute = GPUCompute::new().await;

  println!("==== VULKAN ====");
  println!("Device: {:?}", gdu_compute.get_gpu_name());
  println!("BATCH_COUNT: {}", GPU_BATCH_COUNT);
  println!("GPU_STEP_SIZE: {:#010x}", GPU_STEP_SIZE);
  println!("================");

  let mut time_start = Instant::now();

  let mut total_hash_count: u64 = 0;
  let y_start: u64 = 1;
  let input_data = [0; 16];

  let state = checksum::State::new(0x9191, &data);
  state.print();

  for y in y_start..0xFFFF_FFFF_u64
  {
    gdu_compute.write_input_data(&input_data);

    // checks the entire 1 - 0xFFFF'FFFF range, zero must be ignored
    for offset in (0_u64..0xFFFF_FFFF).step_by(GPU_STEP_SIZE as usize)
    {
      gdu_compute.run(offset as u32, GPU_GROUP_SIZE_XY);
      //println!("GPU: offset: {:#010x}", offset);
    }
    total_hash_count += 0xFFFF_FFFF_u64;

    // To minimize transfers, only check results after a full loop.
    // The success-flag is sticky and will persist.
    let result = gdu_compute.read_output_data().await;

    //println!("GPU: {:?}", result);
    if result[0] != 0
    {
      println!("GPU: Success!");
      return Some(result);
    }

    if y % 4 == 0
    {
      println!("Y: {:#10X} (+{}) | Time: {:?} | Total: {} GHashes",
        y, y-y_start, time_start.elapsed(),
        total_hash_count / 1000_000_000_u64
      );
      time_start = Instant::now();
    }
  }

  None
}

#[derive(Parser)]
struct Cli {
    /// Input ROM to patch
    rom: std::path::PathBuf,
}

fn main() {
  let args = Cli::parse();

  let data_bytes = fs::read(args.rom).unwrap();

  if data_bytes.len() % 4 != 0 {
    println!("Error: ROM size is not a multiple of 4 bytes!");
    return;
  }
  if data_bytes.len() < BOOTCODE_SIZE as usize {
    println!("Error: ROM size is too small, must be at least {} bytes!", BOOTCODE_SIZE);
    return;
  }

  // read file as u32 byteswapped to our native endianness
  let data: Vec<u32> = data_bytes.chunks(4)
    .map(|x| u32::from_be_bytes([x[0], x[1], x[2], x[3]]))
    .collect();

  block_on(bruteforce_run(data));
}
