use ash::vk;
use log::warn;
use std::path::PathBuf;

pub struct Shader {
    pub(crate) module: vk::ShaderModule,
    destroyed: bool,
}

impl Shader {
    pub unsafe fn new_from_file(device: &ash::Device, file: PathBuf) -> anyhow::Result<Shader> {
        Self::new_from_spirv(device, &std::fs::read(file)?)
    }

    pub unsafe fn new_from_spirv(device: &ash::Device, source: &[u8]) -> anyhow::Result<Shader> {
        // Make sure that source is padded to 4 bytes
        assert!(source.len() % 4 == 0);
        let create_info = vk::ShaderModuleCreateInfo::default().code(std::slice::from_raw_parts(
            source.as_ptr() as *const u32,
            source.len() / 4,
        ));

        let module = device.create_shader_module(&create_info, None)?;
        Ok(Shader {
            module,
            destroyed: false,
        })
    }

    pub unsafe fn destroy(&mut self, device: &ash::Device) {
        if !self.destroyed {
            device.destroy_shader_module(self.module, None);
            self.destroyed = true;
        } else {
            warn!("Shader already destroyed");
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        if !self.destroyed {
            warn!("Leaked shader");
        }
    }
}
