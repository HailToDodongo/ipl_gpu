use std::process::{Command};
use std::str;

fn main() {
  let res = Command::new("glslc")
    .args(&["-O", "--target-env=vulkan1.2", "-fshader-stage=compute", "src/shader/shader.glsl", "-o", "src/shader/shader.spv"])
    .output().expect("Failed to compile shader");

  println!("Shader: {}", str::from_utf8(&res.stderr).unwrap());

  std::process::exit(res.status.code().unwrap_or(1));
}