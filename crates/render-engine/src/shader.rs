pub const YUV_TO_RGB_SHADER: &str = r#"
@fragment
fn fs_main(@builtin(position) pos: vec4f) -> @location(0) vec4f {
    return vec4f(0.0, 0.0, 0.0, 1.0);
}
"#;
