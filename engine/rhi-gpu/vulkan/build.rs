use std::process::Command;
use std::path::Path;

fn main() {
    let shader_dir = Path::new("shaders");
    let output_dir = Path::new("../../../assets/shader");
    
    std::fs::create_dir_all(output_dir).ok();
    
    let shaders = [
        ("vert.glsl", "vert.spv", "vert"),
        ("frag.glsl", "frag.spv", "frag"),
    ];
    
    for (input, output, stage) in &shaders {
        let input_path = shader_dir.join(input);
        let output_path = output_dir.join(output);
        
        let status = Command::new("glslc")
            .arg(format!("-fshader-stage={}", stage))
            .arg(&input_path)
            .arg("-o")
            .arg(&output_path)
            .status()
            .expect("Failed to run glslc. Install Vulkan SDK.");
        
        if !status.success() {
            panic!("Shader compilation failed for {}", input);
        }
        
        println!("cargo:rerun-if-changed={}", input_path.display());
    }
}