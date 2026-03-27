// Export module — write mesh to STL/OBJ files.

use std::path::Path;
use std::io::Write;
use crate::renderer::Mesh;

pub fn export_stl(mesh: &Mesh, path: &Path) -> anyhow::Result<()> {
    let mut file = std::fs::File::create(path)?;

    // Binary STL: 80-byte header + u32 triangle count + triangles
    let header = [0u8; 80];
    file.write_all(&header)?;

    let tri_count = (mesh.indices.len() / 3) as u32;
    file.write_all(&tri_count.to_le_bytes())?;

    for tri in mesh.indices.chunks(3) {
        let v0 = glam::Vec3::from(mesh.vertices[tri[0] as usize].position);
        let v1 = glam::Vec3::from(mesh.vertices[tri[1] as usize].position);
        let v2 = glam::Vec3::from(mesh.vertices[tri[2] as usize].position);

        let normal = (v1 - v0).cross(v2 - v0).normalize_or_zero();

        // Normal (3 floats)
        file.write_all(bytemuck::cast_slice(&[normal.x, normal.y, normal.z]))?;
        // Vertex 0
        file.write_all(bytemuck::cast_slice(&[v0.x, v0.y, v0.z]))?;
        // Vertex 1
        file.write_all(bytemuck::cast_slice(&[v1.x, v1.y, v1.z]))?;
        // Vertex 2
        file.write_all(bytemuck::cast_slice(&[v2.x, v2.y, v2.z]))?;
        // Attribute byte count (unused)
        file.write_all(&[0u8; 2])?;
    }

    Ok(())
}

pub fn export_obj(mesh: &Mesh, path: &Path) -> anyhow::Result<()> {
    let mut file = std::fs::File::create(path)?;

    writeln!(file, "# GraniteX OBJ export")?;

    // Write vertices
    for v in &mesh.vertices {
        writeln!(file, "v {} {} {}", v.position[0], v.position[1], v.position[2])?;
    }

    // Write normals
    for v in &mesh.vertices {
        writeln!(file, "vn {} {} {}", v.normal[0], v.normal[1], v.normal[2])?;
    }

    // Write faces (OBJ is 1-indexed)
    for tri in mesh.indices.chunks(3) {
        let i0 = tri[0] + 1;
        let i1 = tri[1] + 1;
        let i2 = tri[2] + 1;
        writeln!(file, "f {}//{} {}//{} {}//{}", i0, i0, i1, i1, i2, i2)?;
    }

    Ok(())
}
