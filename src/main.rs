// @author Max Bebök
// @license MIT

mod gpu;
mod checksum;

use std::{fs};
use std::io::Write;
use std::time::Instant;
use futures::executor::block_on;
use clap::Parser;
use gpu::compute::GPUCompute;

const BOOTCODE_WORD_OFFSET: usize = 0x10;
const BOOTCODE_SIZE: u32 = 0x1000;
const GPU_BATCH_COUNT: u32 = 16*8; // keep in sync with the shader!
const GPU_GROUP_SIZE_XY: u32 = 512;
//const GPU_BATCH_COUNT: u32 = 2; // DEBUG
//const GPU_GROUP_SPLIT: u32 = 2;
const GPU_STEP_SIZE: u32 = GPU_GROUP_SIZE_XY * GPU_GROUP_SIZE_XY * GPU_BATCH_COUNT;

async fn bruteforce_run(seed: u32, offset: u32, data: &mut [u32]) -> bool {

  let gdu_compute = GPUCompute::new().await;

  println!("==== VULKAN ====");
  println!("Device: {:?}", gdu_compute.get_gpu_name());
  println!("BATCH_COUNT: {}", GPU_BATCH_COUNT);
  println!("GPU_STEP_SIZE: {:#010x}", GPU_STEP_SIZE);
  println!("================");

  let mut total_hash_count: u64 = 0;
  let y_start: u64 = offset as u64;

  // Counteract the initial seed
  // This forces state.input[] 1, 7, 8, 14 and 15 to be, and stay zero.
  data[0] = checksum::calc_init_value(seed);

  let mut state = checksum::State::new(seed, &data);
  checksum::calc(&mut state, &data, 1, 1002);

  let mut time_start = Instant::now();
  let starting_state = state.clone();

  for y in y_start..0xFFFF_FFFF_u64
  {
    state = starting_state.clone();

    let buffer_12_compensate = (0-state.buffer[12]) - y as u32;

    data[1003] = buffer_12_compensate; // forces buffer[12] to be zero
    data[1004] = y as u32; // must NOT be zero
    data[1005] = 0; // must be zero!
    data[1006] = 0; // must be zero!

    //                                     input[x]: next, data, last
    checksum::calc_step(&mut state, &data, 1003); // 1003, 1002, 1001
    checksum::calc_step(&mut state, &data, 1004); // 1004, 1003, 1002
    checksum::calc_step(&mut state, &data, 1005); // 1005, 1004, 1003
    checksum::calc_step(&mut state, &data, 1006); // 1006, 1005, 1004
    checksum::calc_step_1007_indep(&mut state); //   ----, 1006, 1005

    debug_assert!(state.buffer[1] == 0); debug_assert!(state.buffer[7] == 0);
    debug_assert!(state.buffer[8] == 0); debug_assert!(state.buffer[12] == 0);
    debug_assert!(state.buffer[14] == 0); debug_assert!(state.buffer[15] == 0);

    gdu_compute.write_input_data(&state.buffer);

    // checks the entire 1 - 0xFFFF'FFFF range, zero must be ignored
    for x in (1_u64..0xFFFF_FFFF).step_by(GPU_STEP_SIZE as usize)
    {
      gdu_compute.run(x as u32, GPU_GROUP_SIZE_XY);
      //println!("GPU: offset: {:#010x}", offset);
    }
    total_hash_count += 0xFFFF_FFFF_u64;

    // To minimize transfers, only check results after a full loop.
    // The success-flag is sticky and will persist.
    let result = gdu_compute.read_output_data().await;
    if result[0] != 0 {
      let x = result[2];
      data[1007] = x;
      println!("Found matching checksum: {:#010x} {:#010x} @ YX: {:#010x} {:#010x}", result[1], result[0], y, x);
      return true;
    }

    if (y-y_start) % 4 == 0
    {
      println!("Y: {:#10X} (+{}) | Time: {:?} | Total: {} GHashes",
        y, y-y_start, time_start.elapsed(),
        total_hash_count / 1000_000_000_u64
      );
      time_start = Instant::now();
    }
  }

  return false
}

#[derive(Parser)]
struct Cli {
    /// Path to the ROM file
    rom: std::path::PathBuf,

    /// Seed to brute-force (use 0x9191)
    #[clap(short, long, value_parser=clap_num::maybe_hex::<u32>)]
    seed: u32,

    /// Offset (Y) to start at, random if now set
    #[clap(short, long, value_parser=clap_num::maybe_hex::<u32>)]
    offset: Option<u32>,
}

fn main() {
  let args = Cli::parse();
  let data_bytes = fs::read(&args.rom).unwrap();

  if data_bytes.len() % 4 != 0 {
    println!("Error: ROM size is not a multiple of 4 bytes!");
    return;
  }
  if data_bytes.len() < BOOTCODE_SIZE as usize {
    println!("Error: ROM size is too small, must be at least {} bytes!", BOOTCODE_SIZE);
    return;
  }

  // read file as u32 byteswapped to our native endianness
  let mut data: Vec<u32> = data_bytes.chunks(4)
    .map(|x| u32::from_be_bytes([x[0], x[1], x[2], x[3]]))
    .collect();

  println!("Start brute-forcing file {:?} for seed {:#06x}", args.rom, args.seed);
  let time_start = Instant::now();

  let offset = args.offset.unwrap_or(fastrand::u32(..0xFFFF_0000));
  let res = block_on(bruteforce_run(args.seed, offset, &mut data[BOOTCODE_WORD_OFFSET..]));

  println!("Total-Time: {:?}", time_start.elapsed());
  if res {
    let file_path_base = args.rom.with_extension("match.z64");
    println!("Writing patched ROM to {:?}", file_path_base);

    let mut file_out = fs::File::create(file_path_base).unwrap();
    let bytes_out = data.iter().flat_map(|x| x.to_be_bytes()).collect::<Vec<u8>>();
    file_out.write_all(&bytes_out).unwrap();
  } else {
    println!("Failed to find matching hash!");
  }
}
