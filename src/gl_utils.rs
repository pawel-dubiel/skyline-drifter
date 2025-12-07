use std::ffi::CString;

pub unsafe fn compile_shader(src: &str, shader_type: gl::types::GLenum) -> u32 {
    let shader = gl::CreateShader(shader_type);
    let c_str = CString::new(src.as_bytes()).unwrap();
    gl::ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
    gl::CompileShader(shader);

    // Check for errors
    let mut success = gl::FALSE as i32;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
    if success != gl::TRUE as i32 {
        let mut len = 0;
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
        let mut buffer = Vec::with_capacity(len as usize);
        buffer.set_len((len as usize) - 1); // skip null terminator
        gl::GetShaderInfoLog(shader, len, std::ptr::null_mut(), buffer.as_mut_ptr() as *mut i8);
        panic!("Shader compilation failed: {}", String::from_utf8_lossy(&buffer));
    }
    shader
}

pub unsafe fn link_program(vs: u32, fs: u32) -> u32 {
    let program = gl::CreateProgram();
    gl::AttachShader(program, vs);
    gl::AttachShader(program, fs);
    gl::LinkProgram(program);
    
    // Check for errors
    let mut success = gl::FALSE as i32;
    gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
    if success != gl::TRUE as i32 {
         let mut len = 0;
        gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
        let mut buffer = Vec::with_capacity(len as usize);
        buffer.set_len((len as usize) - 1);
        gl::GetProgramInfoLog(program, len, std::ptr::null_mut(), buffer.as_mut_ptr() as *mut i8);
        panic!("Program linking failed: {}", String::from_utf8_lossy(&buffer));
    }
    
    gl::DeleteShader(vs);
    gl::DeleteShader(fs);
    program
}
