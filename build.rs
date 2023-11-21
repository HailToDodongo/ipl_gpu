use std::process::Command;

fn main() {
  Command::new("glslc")
    .args(&["-O", "--target-env=vulkan1.2", "-fshader-stage=compute", "src/shader/shader.glsl", "-o", "src/shader/shader.spv"])
    .output()
    .expect("Failed to cmpiler shader.glsl to shader.spv");
}