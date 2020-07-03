mod buffer;
mod camera;
mod debug;
mod fae;
mod instance_device_queues;
mod model;
mod pools_and_command_buffers;
mod render_pass_and_pipeline;
mod surface;
mod swapchain;
use ash::{
    version::{DeviceV1_0, InstanceV1_0},
    vk,
};
use buffer::Buffer;
use debug::FaeDebug;
use fae::Fae;
use instance_device_queues::{
    init_device_and_queues, init_instance, init_physical_device_and_properties, QueueFamilies,
    Queues,
};
use model::{InstanceData, Model};
use pools_and_command_buffers::{create_command_buffers, Pools};
use render_pass_and_pipeline::{init_render_pass, Pipeline};
use surface::FaeSurface;
use swapchain::FaeSwapchain;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use camera::Camera;
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop)?;
    let mut fae = Fae::init(window)?;
    let mut camera = Camera::builder().build();
    let mut sphere = Model::sphere(3);
    sphere.insert_visibly(InstanceData {
        model_matrix: nalgebra::Matrix4::new_scaling(0.5).into(),
        color: [0.5, 0.0, 0.0],
    });
    sphere.update_vertex_buffer(&fae.allocator)?;
    sphere.update_index_buffer(&fae.allocator)?;
    sphere.update_instance_buffer(&fae.allocator)?;
    fae.models = vec![sphere];
    use winit::event::{Event, WindowEvent};
    event_loop.run(move |event, _, controlflow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *controlflow = winit::event_loop::ControlFlow::Exit;
        }
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } => {
            if let winit::event::KeyboardInput {
                state: winit::event::ElementState::Pressed,
                virtual_keycode: Some(keycode),
                ..
            } = input
            {
                match keycode {
                    winit::event::VirtualKeyCode::Right => {
                        camera.turn_right(0.1);
                    }
                    winit::event::VirtualKeyCode::Left => {
                        camera.turn_left(0.1);
                    }
                    winit::event::VirtualKeyCode::S => {
                        camera.move_backward(0.05);
                    }
                    winit::event::VirtualKeyCode::W => {
                        camera.move_forward(0.05);
                    }
                    winit::event::VirtualKeyCode::A => {
                        camera.move_left(0.05);
                    }
                    winit::event::VirtualKeyCode::D => {
                        camera.move_right(0.05);
                    }
                    winit::event::VirtualKeyCode::Up => {
                        camera.turn_up(0.02);
                    }
                    winit::event::VirtualKeyCode::Down => {
                        camera.turn_down(0.02);
                    }
                    _ => {}
                }
            }
        }
        Event::MainEventsCleared => {
            fae.window.request_redraw();
        }
        Event::RedrawRequested(_) => {
            // aquire the next image
            let (image_index, _) = unsafe {
                fae.swapchain
                    .swapchain_loader
                    .acquire_next_image(
                        fae.swapchain.swapchain,
                        std::u64::MAX,
                        fae.swapchain.image_available[fae.swapchain.current_image],
                        vk::Fence::null(),
                    )
                    .expect("image aquisition trouble")
            };
            unsafe {
                // wait for fence
                fae.device
                    .wait_for_fences(
                        &[fae.swapchain.may_begin_drawing[fae.swapchain.current_image]],
                        true,
                        std::u64::MAX,
                    )
                    .expect("fence-waiting");
                // reset fence
                fae.device
                    .reset_fences(&[fae.swapchain.may_begin_drawing[fae.swapchain.current_image]])
                    .expect("resetting fences");
            };
            for m in &mut fae.models {
                camera.update_buffer(&fae.allocator, &mut fae.uniform_buffer);
                m.update_instance_buffer(&fae.allocator).unwrap();
            }
            //update command buffer
            fae.update_command_buffer(image_index as usize)
                .expect("updateing the command buffer");
            // command buffer setup info
            let semaphores_available = [fae.swapchain.image_available[fae.swapchain.current_image]];
            let waiting_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let semaphores_finished =
                [fae.swapchain.rendering_finished[fae.swapchain.current_image]];
            let command_buffers = [fae.command_buffers[image_index as usize]];
            let submit_info = [vk::SubmitInfo::builder()
                .wait_semaphores(&semaphores_available)
                .wait_dst_stage_mask(&waiting_stages)
                .command_buffers(&command_buffers)
                .signal_semaphores(&semaphores_finished)
                .build()];
            // submit command buffer
            unsafe {
                fae.device
                    .queue_submit(
                        fae.queues.graphics_queue,
                        &submit_info,
                        fae.swapchain.may_begin_drawing[fae.swapchain.current_image],
                    )
                    .expect("queue submission");
            };
            // present image to screen
            let swapchains = [fae.swapchain.swapchain];
            let indices = [image_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&semaphores_finished)
                .swapchains(&swapchains)
                .image_indices(&indices);
            unsafe {
                fae.swapchain
                    .swapchain_loader
                    .queue_present(fae.queues.graphics_queue, &present_info)
                    .expect("queue presentation");
            };
        }
        _ => {}
    });
}
