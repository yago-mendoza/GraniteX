// STL importer — supports binary and ASCII formats.
//
// Binary STL: 80-byte header, u32 triangle count, then 50 bytes per triangle.
// ASCII STL: "solid" header, facet/vertex text blocks, "endsolid" footer.

use std::path::Path;

use glam::Vec3;
use crate::renderer::Mesh;
use super::ImportError;

pub fn load(path: &Path) -> Result<Mesh, ImportError> {
    let data = std::fs::read(path)?;
    if is_ascii(&data) {
        parse_ascii(&data)
    } else {
        parse_binary(&data)
    }
}

fn is_ascii(data: &[u8]) -> bool {
    if data.len() < 6 { return false; }
    let header = std::str::from_utf8(&data[..6]).unwrap_or("");
    header.starts_with("solid") && data.iter().take(1000).all(|&b| b < 128)
}

fn parse_binary(data: &[u8]) -> Result<Mesh, ImportError> {
    if data.len() < 84 {
        return Err(ImportError::Parse("Binary STL too short".into()));
    }

    let tri_count = u32::from_le_bytes([data[80], data[81], data[82], data[83]]) as usize;
    let expected = tri_count.checked_mul(50)
        .and_then(|v| v.checked_add(84))
        .ok_or_else(|| ImportError::Parse("STL triangle count overflow".into()))?;
    if data.len() < expected {
        return Err(ImportError::Parse(format!(
            "Binary STL truncated: expected {} bytes, got {}", expected, data.len()
        )));
    }

    let mut positions = Vec::with_capacity(tri_count * 3);
    let mut normals = Vec::with_capacity(tri_count * 3);
    let mut indices = Vec::with_capacity(tri_count * 3);

    for i in 0..tri_count {
        let offset = 84 + i * 50;
        let n = read_vec3(data, offset);
        let v0 = read_vec3(data, offset + 12);
        let v1 = read_vec3(data, offset + 24);
        let v2 = read_vec3(data, offset + 36);

        let base = (i * 3) as u32;
        positions.push(v0);
        positions.push(v1);
        positions.push(v2);
        normals.push(n);
        normals.push(n);
        normals.push(n);
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
    }

    Ok(Mesh::from_triangles(&positions, &normals, &indices))
}

fn parse_ascii(data: &[u8]) -> Result<Mesh, ImportError> {
    let text = std::str::from_utf8(data)
        .map_err(|e| ImportError::Parse(format!("Invalid UTF-8: {}", e)))?;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut current_normal = Vec3::ZERO;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("facet normal") {
            current_normal = parse_vec3(rest)?;
        } else if let Some(rest) = line.strip_prefix("vertex") {
            let v = parse_vec3(rest)?;
            let idx = positions.len() as u32;
            positions.push(v);
            normals.push(current_normal);
            indices.push(idx);
        }
    }

    if positions.is_empty() {
        return Err(ImportError::Parse("No triangles found in ASCII STL".into()));
    }

    Ok(Mesh::from_triangles(&positions, &normals, &indices))
}

fn read_vec3(data: &[u8], offset: usize) -> Vec3 {
    let x = f32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
    let y = f32::from_le_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]);
    let z = f32::from_le_bytes([data[offset+8], data[offset+9], data[offset+10], data[offset+11]]);
    Vec3::new(x, y, z)
}

fn parse_vec3(s: &str) -> Result<Vec3, ImportError> {
    let parts: Vec<f32> = s.split_whitespace()
        .filter_map(|p| p.parse().ok())
        .collect();
    if parts.len() < 3 {
        return Err(ImportError::Parse(format!("Expected 3 floats, got: '{}'", s.trim())));
    }
    Ok(Vec3::new(parts[0], parts[1], parts[2]))
}
