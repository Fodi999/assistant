fn main() {
    let wgsl = vec![
        include_str!("src/web/home/scripts/webgpu/shader/uniforms.rs"),
        include_str!("src/web/home/scripts/webgpu/shader/background.rs"),
        include_str!("src/web/home/scripts/webgpu/shader/geometry.rs"),
        include_str!("src/web/home/scripts/webgpu/shader/sdf.rs"),
        include_str!("src/web/home/scripts/webgpu/shader/particles_vert.rs"),
        include_str!("src/web/home/scripts/webgpu/shader/particles_frag.rs")
    ].join("\n");
    // We just want to find where missing } is. Let's just output the text:
    println!("{}", wgsl);
}
