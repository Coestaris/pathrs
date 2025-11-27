use crate::tracer::Bundle;
use ash::vk;
use log::warn;

pub struct CommandBuffer {
    command_buffer: vk::CommandBuffer,
    destroyed: bool,
}

impl CommandBuffer {
    pub unsafe fn new_from_pool(
        bundle: Bundle,
        command_pool: vk::CommandPool,
    ) -> anyhow::Result<Self> {
        let cmd_alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = bundle.device.allocate_command_buffers(&cmd_alloc_info)?[0];
        Ok(Self {
            command_buffer,
            destroyed: false,
        })
    }

    pub unsafe fn begin(&self, bundle: Bundle) -> anyhow::Result<()> {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        bundle
            .device
            .begin_command_buffer(self.command_buffer, &begin_info)?;
        Ok(())
    }

    pub unsafe fn end(&self, bundle: Bundle) -> anyhow::Result<()> {
        bundle.device.end_command_buffer(self.command_buffer)?;
        Ok(())
    }

    pub unsafe fn destroy(&mut self, bundle: Bundle, pool: vk::CommandPool) {
        if !self.destroyed {
            bundle
                .device
                .free_command_buffers(pool, &[self.command_buffer]);
            self.destroyed = true;
        } else {
            warn!("CommandBuffer already destroyed");
        }
    }

    pub unsafe fn reset(&self, bundle: Bundle) -> anyhow::Result<()> {
        bundle
            .device
            .reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty())?;
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
