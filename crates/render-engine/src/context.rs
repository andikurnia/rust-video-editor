#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("GPU not available")]
    NoGpu,
    #[error("Shader compilation failed: {0}")]
    ShaderError(String),
    #[error("Render failed: {0}")]
    Failed(String),
}

pub struct RenderContext;

impl RenderContext {
    pub fn new() -> Result<Self, RenderError> {
        Err(RenderError::NoGpu)
    }
}
