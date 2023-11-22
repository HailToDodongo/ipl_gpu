// @author Max Bebök
// @license MIT

use wgpu::{BufferAddress, BufferUsages};

pub fn create_pair(device: &wgpu::Device, size: usize, readonly: bool, name: &str) -> (wgpu::Buffer, wgpu::Buffer)
{
  let host_usage = if readonly
     {BufferUsages::MAP_WRITE | BufferUsages::COPY_SRC} else
     {BufferUsages::MAP_READ | BufferUsages::COPY_DST};

  let buffer_host = device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
    size: size as BufferAddress,
    usage: host_usage,
    mapped_at_creation: false,
  });
  let buffer_gpu = device.create_buffer(&wgpu::BufferDescriptor {
    label: Some(name),
    size: buffer_host.size(),
    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
    mapped_at_creation: false,
  });

  return (buffer_host, buffer_gpu);
}