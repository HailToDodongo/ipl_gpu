use std::mem::size_of;
use futures::executor::block_on;
use wgpu::{BufferAddress, BufferUsages, include_spirv};

const GPU_BATCH_COUNT: u32 = 128; // 99% GPU
const GPU_GROUP_SPLIT: u32 = 512;
//const GPU_BATCH_COUNT: u32 = 2; // DEBUG
//const GPU_GROUP_SPLIT: u32 = 2;

const GPU_GROUP_SIZE: u32 = GPU_GROUP_SPLIT * GPU_GROUP_SPLIT;
const GPU_STEP_SIZE: u64 = (GPU_GROUP_SIZE * GPU_BATCH_COUNT) as u64;

#[derive(Clone, Copy)]
#[repr(C)]
struct BatchParams {
  offset: u32,
}

async fn run() {
  let steps = execute_gpu().await.unwrap();
  let steps_str = steps.iter().map(|x| x.to_string()).collect::<Vec<String>>();
  println!("Steps: [{}]", steps_str.join(", "));
}

async fn execute_gpu() -> Option<Vec<u32>>
{
  let instance = wgpu::Instance::default();
  let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await?;
  let adapter_info = adapter.get_info();

  println!("==== VULKAN ====");
  println!("Device: {:?}", adapter_info.name);
  println!("BATCH_COUNT: {}", GPU_BATCH_COUNT);
  println!("GROUP_SIZE: {}", GPU_GROUP_SIZE);
  println!("GPU_STEP_SIZE: {:#010x}", GPU_STEP_SIZE);
  println!("================");

  let (device, queue) = adapter
    .request_device(
      &wgpu::DeviceDescriptor {
        label: None,
        features: wgpu::Features::empty(),
        limits: wgpu::Limits::downlevel_defaults(),
      },
      None,
    ).await.unwrap();

  execute_gpu_inner(&device, &queue).await
}

fn create_gpu_buffer(device: &wgpu::Device, size: usize, readonly: bool, name: &str) -> (wgpu::Buffer, wgpu::Buffer)
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

async fn execute_gpu_inner(
  device: &wgpu::Device,
  queue: &wgpu::Queue
) -> Option<Vec<u32>> {

  let cs_module = device.create_shader_module(include_spirv!("shader.spv"));

  let input_data = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16];
  let offset = 0u32;

  let (_buffer_input, buffer_input_gpu) = create_gpu_buffer(device, input_data.len() * size_of::<u32>()*2, true,"Input");
  let (buffer_checksum, buffer_checksum_gpu) = create_gpu_buffer(device, size_of::<u32>()*2, false, "Checksum");
  let (_buffer_result, buffer_result_gpu) = create_gpu_buffer(device, size_of::<u32>(), false, "Result");
  let (_buffer_offset, buffer_offset_gpu) = create_gpu_buffer(device, size_of::<u32>(), false, "Offset");

  let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
    label: None, layout: None,
    module: &cs_module,
    entry_point: "main",
  });

  // Instantiates the bind group, once again specifying the binding of buffers.
  let bind_group_layout = compute_pipeline.get_bind_group_layout(0);
  let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: None,
    layout: &bind_group_layout,
    entries: &[
      wgpu::BindGroupEntry { binding: 0, resource: buffer_input_gpu.as_entire_binding() },
      wgpu::BindGroupEntry { binding: 1, resource: buffer_checksum_gpu.as_entire_binding() },
      wgpu::BindGroupEntry { binding: 2, resource: buffer_result_gpu.as_entire_binding() },
      wgpu::BindGroupEntry { binding: 3, resource: buffer_offset_gpu.as_entire_binding() },
    ],
  });

  // A command encoder executes one or many pipelines.
  // It is to WebGPU what a command buffer is to Vulkan.
  let mut encoder =
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
  {
    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
      label: None, timestamp_writes: None,
    });

    //let bytes: [u8; 4] = [1,2,3,4];
    //cpass.set_push_constants(0, &bytes);
    cpass.set_pipeline(&compute_pipeline);
    cpass.set_bind_group(0, &bind_group, &[]);
    cpass.dispatch_workgroups(input_data.len() as u32, 1, 1); // Number of cells to run, the (x,y,z) size of item being processed
  }

  encoder.copy_buffer_to_buffer(
    &buffer_checksum_gpu, 0,
    &buffer_checksum, 0,
    buffer_checksum.size()
  );

  queue.write_buffer(&buffer_input_gpu, 0, bytemuck::cast_slice(&[input_data]));
  queue.write_buffer(&buffer_offset_gpu, 0, bytemuck::cast_slice(&[offset]));
  queue.submit(Some(encoder.finish()));

  // Note that we're not calling `.await` here.
  let buffer_slice = buffer_checksum.slice(..);
  // Sets the buffer up for mapping, sending over the result of the mapping back to us when it is finished.
  let (sender, receiver) = flume::bounded(1);
  buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

  device.poll(wgpu::Maintain::Wait); // blocking

  // Awaits until `buffer_future` can be read from
  if let Ok(Ok(())) = receiver.recv_async().await
  {
    // Gets contents of buffer
    let data = buffer_slice.get_mapped_range();
    // Since contents are got in bytes, this converts these bytes back to u32
    let result = bytemuck::cast_slice(&data).to_vec();

    // With the current interface, we have to make sure all mapped views are
    // dropped before we unmap the buffer.
    drop(data);
    buffer_checksum.unmap(); // Unmaps buffer from memory

    // Returns data from buffer
    Some(result)
  } else {
    panic!("failed to run compute on gpu!")
  }
}

fn main() {
  block_on(run());
}
