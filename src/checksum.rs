// @author Max Bebök
// @license MIT

pub(crate) struct State {
  pub buffer: [u32; 16],
}

const MAGIC_NUMBER: u32 = 0x6c078965;

impl State {
  pub fn new(seed: u32, input: &Vec<u32>) -> Self {
    let init: u32 = MAGIC_NUMBER * (seed & 0xFF) + 1;
    return State {buffer: [init ^ input[0]; 16]}
  }

  pub fn print(&self) {
    println!("BUFF: {:#10X} {:#10X} {:#10X} {:#10X} | {:#10X} {:#10X} {:#10X} {:#10X} | {:#10X} {:#10X} {:#10X} {:#10X} | {:#10X} {:#10X} {:#10X} {:#10X}",
      self.buffer[0],  self.buffer[1],  self.buffer[2],  self.buffer[3],
      self.buffer[4],  self.buffer[5],  self.buffer[6],  self.buffer[7],
      self.buffer[8],  self.buffer[9],  self.buffer[10], self.buffer[11],
      self.buffer[12], self.buffer[13], self.buffer[14], self.buffer[15]
    );
  }
}