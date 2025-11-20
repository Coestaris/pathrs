use ash::vk;
use log::warn;

pub struct CommandBuffer {
    command_buffer: vk::CommandBuffer,
    destroyed: bool,
}

impl CommandBuffer {
    pub unsafe fn new_from_pool(
        device: &ash::Device,
        command_pool: vk::CommandPool,
    ) -> anyhow::Result<Self> {
        let cmd_alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = device.allocate_command_buffers(&cmd_alloc_info)?[0];
        Ok(Self {
            command_buffer,
            destroyed: false,
        })
    }

    pub unsafe fn begin(&self, device: &ash::Device) -> anyhow::Result<()> {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        device.begin_command_buffer(self.command_buffer, &begin_info)?;
        Ok(())
    }

    pub unsafe fn end(&self, device: &ash::Device) -> anyhow::Result<()> {
        device.end_command_buffer(self.command_buffer)?;
        Ok(())
    }

    pub unsafe fn begin_renderpass(
        &self,
        device: &ash::Device,
        render_pass_begin_info: &vk::RenderPassBeginInfo,
        contents: vk::SubpassContents,
    ) {
        device.cmd_begin_render_pass(self.command_buffer, render_pass_begin_info, contents);
    }

    pub unsafe fn end_renderpass(&self, device: &ash::Device) {
        device.cmd_end_render_pass(self.command_buffer);
    }

    pub unsafe fn copy_buffer(
        &self,
        device: &ash::Device,
        src_buffer: vk::Buffer,
        dst_buffer: vk::Buffer,
        size: vk::DeviceSize,
    ) {
        let copy_region = vk::BufferCopy::default().size(size);
        device.cmd_copy_buffer(self.command_buffer, src_buffer, dst_buffer, &[copy_region]);
    }

    pub unsafe fn set_viewport(&self, device: &ash::Device, viewport: vk::Viewport) {
        device.cmd_set_viewport(self.command_buffer, 0, &[viewport]);
    }

    pub unsafe fn set_scissor(&self, device: &ash::Device, scissor: vk::Rect2D) {
        device.cmd_set_scissor(self.command_buffer, 0, &[scissor]);
    }

    pub unsafe fn bind_vertex_buffer(
        &self,
        device: &ash::Device,
        first_binding: u32,
        buffer: vk::Buffer,
        offset: vk::DeviceSize,
    ) {
        device.cmd_bind_vertex_buffers(self.command_buffer, first_binding, &[buffer], &[offset]);
    }

    pub unsafe fn bind_index_buffer(
        &self,
        device: &ash::Device,
        buffer: vk::Buffer,
        offset: vk::DeviceSize,
        index_type: vk::IndexType,
    ) {
        device.cmd_bind_index_buffer(self.command_buffer, buffer, offset, index_type);
    }

    pub unsafe fn draw_indexed(
        &self,
        device: &ash::Device,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        device.cmd_draw_indexed(
            self.command_buffer,
            index_count,
            instance_count,
            first_index,
            vertex_offset,
            first_instance,
        );
    }

    pub unsafe fn bind_pipeline(
        &self,
        device: &ash::Device,
        bind: vk::PipelineBindPoint,
        pipeline: vk::Pipeline,
    ) {
        device.cmd_bind_pipeline(self.command_buffer, bind, pipeline);
    }

    pub unsafe fn bind_descriptor_sets(
        &self,
        device: &ash::Device,
        bind: vk::PipelineBindPoint,
        pipeline_layout: vk::PipelineLayout,
        first_set: u32,
        descriptor_sets: &[vk::DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        device.cmd_bind_descriptor_sets(
            self.command_buffer,
            bind,
            pipeline_layout,
            first_set,
            descriptor_sets,
            dynamic_offsets,
        );
    }

    pub unsafe fn dispatch(
        &self,
        device: &ash::Device,
        group_count_x: u32,
        group_count_y: u32,
        group_count_z: u32,
    ) {
        device.cmd_dispatch(
            self.command_buffer,
            group_count_x,
            group_count_y,
            group_count_z,
        );
    }

    pub unsafe fn destroy(&mut self, pool: vk::CommandPool, device: &ash::Device) {
        if !self.destroyed {
            device.free_command_buffers(pool, &[self.command_buffer]);
            self.destroyed = true;
        } else {
            warn!("CommandBuffer already destroyed");
        }
    }

    pub unsafe fn reset(&self, device: &ash::Device) -> anyhow::Result<()> {
        device.reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty())?;
        Ok(())
    }

    pub fn as_submit_info(&self) -> vk::SubmitInfo<'_> {
        vk::SubmitInfo::default().command_buffers(std::slice::from_ref(&self.command_buffer))
    }

    pub fn as_inner(&self) -> vk::CommandBuffer {
        self.command_buffer
    }
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        if !self.destroyed {
            warn!("Leaked CommandBuffer");
        }
    }
}
