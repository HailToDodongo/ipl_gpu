// @author Max Bebök
// @license MIT

use std::mem::size_of;
use futures_intrusive::channel;
use wgpu::{Dx12Compiler, Gles3MinorVersion, include_spirv_raw, InstanceFlags};
use crate::gpu::{buffer, layout};

const CMD_ENCODER_DESC: wgpu::CommandEncoderDescriptor = wgpu::CommandEncoderDescriptor { label: None };
const COMPUTE_PASS_DESC: wgpu::ComputePassDescriptor = wgpu::ComputePassDescriptor { label: None, timestamp_writes: None };

pub(crate) struct GPUCompute {
  //instance: wgpu::Instance,
  //module: ShaderModule,
  queue: wgpu::Queue,
  device: wgpu::Device,
  compute_pipeline: wgpu::ComputePipeline,
  bind_group: wgpu::BindGroup,

  buffer_input_gpu: wgpu::Buffer,
  buffer_checksum: wgpu::Buffer,
  buffer_checksum_gpu: wgpu::Buffer,

  gpu_name: String,
}

impl GPUCompute {
  pub async fn new() -> Self
  {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        flags: InstanceFlags::default(),
        dx12_shader_compiler: Dx12Compiler::default(),
        gles_minor_version: Gles3MinorVersion::default(),
    });

    instance.enumerate_adapters(wgpu::Backends::all()).for_each(|adapter| {
      println!("Adapter: {:?}, Backend: {:?}", adapter.get_info().name, adapter.get_info().backend);
    });

    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.unwrap();
    let adapter_info = adapter.get_info();

    let mut limits = wgpu::Limits::downlevel_defaults();
    limits.max_push_constant_size = 4;

    // request a physical device with the features we need
    let (device, queue) = adapter
      .request_device(
      &wgpu::DeviceDescriptor {
        label: None,
        features: wgpu::Features::SPIRV_SHADER_PASSTHROUGH | wgpu::Features::PUSH_CONSTANTS,
        limits,
      },
      None,
    ).await.unwrap();

    // create the shader module (@TODO: use the safe version once 'umulExtended' is supported)
    let module = unsafe {
      device.create_shader_module_spirv(&include_spirv_raw!("../shader/shader.spv"))
    };
    //let module = device.create_shader_module(include_spirv!("shader/shader.spv"));

    // describe the layout of buffers in the shader...
    let group_layout = layout::create_bind_group_layout(&device);

    // ... and push-constant size (keep both in sync with the GLSL shader!)
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: None,
      bind_group_layouts: &[&group_layout],
      push_constant_ranges: &[wgpu::PushConstantRange {
        stages: wgpu::ShaderStages::COMPUTE,
        range: 0..size_of::<u32>() as u32,
      }],
    });

    // actual pipeline to perform operations with
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
      label: None,
      module: &module,
      entry_point: "main",
      layout: Some(&pipeline_layout),
    });

    // buffer to pass data to/from the shader
    let (_, buffer_input_gpu) = buffer::create_pair(&device, size_of::<u32>()*16, true,"Input");
    let (buffer_checksum, buffer_checksum_gpu) = buffer::create_pair(&device, size_of::<u32>()*4, false, "Checksum/Result");

    // no we bind the buffer to the pipeline/layout
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: None,
      layout: &group_layout,
      entries: &[
        wgpu::BindGroupEntry { binding: 0, resource: buffer_input_gpu.as_entire_binding() },
        wgpu::BindGroupEntry { binding: 1, resource: buffer_checksum_gpu.as_entire_binding() },
      ],
    });

    Self {
      queue, gpu_name: adapter_info.name, device,
      buffer_input_gpu, buffer_checksum_gpu, buffer_checksum,
      bind_group, compute_pipeline
    }
  }

  pub fn write_input_data(&self, data: &[u32])
  {
    self.queue.write_buffer(&self.buffer_input_gpu, 0, bytemuck::cast_slice(&data));
  }

  pub fn run(&self, offset: u32, group_size_xy: u32)
  {
    let mut encoder = self.device.create_command_encoder(&CMD_ENCODER_DESC);
    {
      let mut cpass = encoder.begin_compute_pass(&COMPUTE_PASS_DESC);
      cpass.set_pipeline(&self.compute_pipeline);
      cpass.set_bind_group(0, &self.bind_group, &[]);
      cpass.set_push_constants(0, bytemuck::bytes_of(&offset));
      cpass.dispatch_workgroups(group_size_xy, group_size_xy, 1);
    }
    self.queue.submit(Some(encoder.finish()));
  }

  pub async fn read_output_data(&self) -> Vec<u32>
  {
    let mut encoder = self.device.create_command_encoder(&CMD_ENCODER_DESC);
    // fetch buffer from gpu
    encoder.copy_buffer_to_buffer(
      &self.buffer_checksum_gpu, 0,
      &self.buffer_checksum, 0,
      self.buffer_checksum.size()
    );
    self.queue.submit(Some(encoder.finish()));

    // setup and map buffer for reading
    let buffer_slice = self.buffer_checksum.slice(..);
    let (sender, receiver) = channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());

    // blocking wait for the buffer to be ready
    self.device.poll(wgpu::Maintain::Wait);
    let res_vec: Vec<u32>;

    // actual buffer read into the vector
    receiver.receive().await.unwrap().unwrap();
    {
      let data = buffer_slice.get_mapped_range();
      res_vec = bytemuck::cast_slice(&data).to_vec();
    }
    self.buffer_checksum.unmap();
    return res_vec;
  }

  pub fn get_gpu_name(&self) -> String
  {
    self.gpu_name.clone()
  }
}