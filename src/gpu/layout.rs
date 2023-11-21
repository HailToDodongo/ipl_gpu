// @author Max Bebök
// @license MIT

use std::num::{NonZeroU32, NonZeroU64};
use wgpu::BufferSize;

/// Returns the bind group layout for the compute shader.
/// This needs to stay in sync with the GLSL shader.
pub(crate) fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout
{
  return device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[
      wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Storage { read_only: false },
          has_dynamic_offset: false,
          //min_binding_size: BufferSize::new(16*4),
          min_binding_size: None,
        },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Storage { read_only: false },
          has_dynamic_offset: false,
          //min_binding_size: BufferSize::new(2*4),
          min_binding_size: None,
        },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: 2,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Storage { read_only: false },
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: 3,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Storage { read_only: false },
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      },
    ]
  });
}