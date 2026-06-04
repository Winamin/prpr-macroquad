use miniquad::Context;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShaderVersion {
    GL100, // OpenGL ES 2.0 / WebGL 1.0
    GL300, // OpenGL ES 3.0 / WebGL 2.0
    GL400, // OpenGL 4.0+
}

impl ShaderVersion {
    pub fn detect(_ctx: &Context) -> Self {
        #[cfg(feature = "gl-40")]
        {
            if cfg!(not(target_arch = "wasm32"))
                && cfg!(not(target_os = "android"))
                && cfg!(not(target_os = "ios"))
            {
                return ShaderVersion::GL400;
            }
        }

        #[cfg(feature = "gl-30")]
        {
            return ShaderVersion::GL300;
        }
        ShaderVersion::GL100
    }

    pub fn version_directive(&self) -> &'static str {
        match self {
            ShaderVersion::GL100 => "#version 100",
            ShaderVersion::GL300 => "#version 300 es",
            ShaderVersion::GL400 => "#version 400 core",
        }
    }

    pub fn supports_features(&self) -> ShaderFeatures {
        match self {
            ShaderVersion::GL100 => ShaderFeatures {
                ubo: false,
                ssbo: false,
                compute: false,
                texture_lod: false,
                texture_array: false,
                instancing: false,
            },
            ShaderVersion::GL300 => ShaderFeatures {
                ubo: true,
                ssbo: false,
                compute: false,
                texture_lod: true,
                texture_array: true,
                instancing: true,
            },
            ShaderVersion::GL400 => ShaderFeatures {
                ubo: true,
                ssbo: true,
                compute: true,
                texture_lod: true,
                texture_array: true,
                instancing: true,
            },
        }
    }
}

pub struct ShaderFeatures {
    pub ubo: bool,
    pub ssbo: bool,
    pub compute: bool,
    pub texture_lod: bool,
    pub texture_array: bool,
    pub instancing: bool,
}