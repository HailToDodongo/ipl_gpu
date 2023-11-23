// @author Max Bebök
// @license MIT

#[derive(Clone, Copy)]
pub(crate) struct State {
  pub buffer: [u32; 16],
}

const MAGIC_NUMBER: u32 = 0x6c078965;

impl State {
  pub fn new(seed: u32, data: &[u32]) -> Self {
    let init: u32 = calc_init_value(seed);
    return State {buffer: [init ^ data[0]; 16]}
  }

  #[allow(dead_code)]
  pub fn print(&self) {
    println!("BUFF: {:#10X} {:#10X} {:#10X} {:#10X} | {:#10X} {:#10X} {:#10X} {:#10X} | {:#10X} {:#10X} {:#10X} {:#10X} | {:#10X} {:#10X} {:#10X} {:#10X}",
      self.buffer[0],  self.buffer[1],  self.buffer[2],  self.buffer[3],
      self.buffer[4],  self.buffer[5],  self.buffer[6],  self.buffer[7],
      self.buffer[8],  self.buffer[9],  self.buffer[10], self.buffer[11],
      self.buffer[12], self.buffer[13], self.buffer[14], self.buffer[15]
    );
  }
}

fn hash_mul_diff(factor_base: u32, mut factor_main: u32, factor_alt: u32) -> u32 {
      // for factor_main == 0, always returns zero!
    if factor_main == 0 { factor_main = factor_alt; }

    let prod = factor_base as u64 * factor_main as u64;
    let diff = (prod >> 32) as u32 - prod as u32;

    if diff == 0 { factor_base } else { diff }
}

/// Value for the first word, used to exploit a zero-init state.
pub fn calc_init_value(seed: u32) -> u32 {
  return MAGIC_NUMBER * (seed & 0xFF) + 1;
}

// The last 2 steps are done on the GPU, the first half of step 1007 can be done
// on the CPU since it is independent of the input data.
pub fn calc_step_1007_indep(state: &mut State)
{
  state.buffer[3] += hash_mul_diff(5, MAGIC_NUMBER, 1007);

  if 0 < state.buffer[6] {
    state.buffer[6] = (state.buffer[3] + state.buffer[6]) ^ (1007);
  } else {
    state.buffer[6] = (state.buffer[4]) ^ state.buffer[6];
  }
}

// Generic form of a single checksum loop, the above function is a special case.
// This assumes a zero init state for state.buffer, and removed unused code
pub fn calc_step(state: &mut State, data: &[u32], round: u32) 
{
  let data_last = data[(if round == 1 { 0 } else {round - 2}) as usize];
  let data_curr = data[(round - 1) as usize];

  state.buffer[0] += hash_mul_diff(1007 - round, data_curr, round);
  state.buffer[2] ^= data_curr;
  state.buffer[3] += hash_mul_diff(data_curr + 5, MAGIC_NUMBER, round);
  state.buffer[4] += data_curr.rotate_right(data_last & 0x1F);
  state.buffer[5] += data_curr.rotate_left(data_last >> 27);
  if data_curr < state.buffer[6] {
    state.buffer[6] = (state.buffer[3] + state.buffer[6]) ^ (data_curr + round);
  } else {
    state.buffer[6] = (state.buffer[4] + data_curr) ^ state.buffer[6];
  }

  if data_last < data_curr
  {
    state.buffer[9] = hash_mul_diff(state.buffer[9], data_curr, round);
  } else {
    state.buffer[9] += data_curr;
  }

  if round == 1008 { return; }

  let data_next = data[round as usize];
  state.buffer[10] = hash_mul_diff(state.buffer[10] + data_curr, data_next, round);
  state.buffer[11] = hash_mul_diff(state.buffer[11] ^ data_curr, data_next, round);
  state.buffer[12] += data_curr;

  state.buffer[13] += data_curr.rotate_right(data_curr & 0x1F)
                   + data_next.rotate_right(data_next & 0x1F);
}

pub fn calc(state: &mut State, data: &[u32], round_start: u32, round_end: u32)
{
  for round in round_start..=round_end {
    calc_step(state, data, round);
  }
}