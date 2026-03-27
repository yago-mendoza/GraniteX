// Project save/load — .gnx format (JSON-based).

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::renderer::mesh::Mesh;
use crate::renderer::vertex::Vertex;

#[derive(Serialize, Deserialize)]
pub struct ProjectFile {
    pub version: u32,
    pub mesh: MeshData,
    pub camera: CameraData,
}

#[derive(Serialize, Deserialize)]
pub struct MeshData {
    pub vertices: Vec<VertexData>,
    pub indices: Vec<u32>,
    pub next_face_id: u32,
    pub stored_boundaries: HashMap<u32, Vec<[f32; 3]>>,
    pub stored_holes: HashMap<u32, Vec<Vec<[f32; 3]>>>,
}

#[derive(Serialize, Deserialize)]
pub struct VertexData {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub face_id: u32,
}

#[derive(Serialize, Deserialize)]
pub struct CameraData {
    pub target: [f32; 3],
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

pub fn save_project(path: &Path, mesh: &Mesh, camera_state: CameraData) -> Result<()> {
    let vertices: Vec<VertexData> = mesh.vertices.iter().map(|v| VertexData {
        position: v.position,
        normal: v.normal,
        face_id: v.face_id,
    }).collect();

    let stored_boundaries: HashMap<u32, Vec<[f32; 3]>> = mesh.stored_boundaries()
        .iter()
        .map(|(k, v)| (*k, v.iter().map(|p| [p.x, p.y, p.z]).collect()))
        .collect();

    let stored_holes: HashMap<u32, Vec<Vec<[f32; 3]>>> = mesh.stored_holes()
        .iter()
        .map(|(k, v)| (*k, v.iter().map(|hole| hole.iter().map(|p| [p.x, p.y, p.z]).collect()).collect()))
        .collect();

    let project = ProjectFile {
        version: 1,
        mesh: MeshData {
            vertices,
            indices: mesh.indices.clone(),
            next_face_id: mesh.next_face_id(),
            stored_boundaries,
            stored_holes,
        },
        camera: camera_state,
    };

    let json = serde_json::to_string_pretty(&project)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn load_project(path: &Path) -> Result<(Mesh, CameraData)> {
    let json = std::fs::read_to_string(path)?;
    let project: ProjectFile = serde_json::from_str(&json)?;

    if project.version != 1 {
        anyhow::bail!("Unsupported project version {} (expected 1)", project.version);
    }

    if project.mesh.vertices.is_empty() || project.mesh.indices.is_empty() {
        anyhow::bail!("Project file contains empty mesh");
    }

    let vertices: Vec<Vertex> = project.mesh.vertices.iter().map(|v| Vertex {
        position: v.position,
        normal: v.normal,
        face_id: v.face_id,
        _pad: 0,
    }).collect();

    let stored_boundaries: HashMap<u32, Vec<glam::Vec3>> = project.mesh.stored_boundaries
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().map(|p| glam::Vec3::from(p)).collect()))
        .collect();

    let stored_holes: HashMap<u32, Vec<Vec<glam::Vec3>>> = project.mesh.stored_holes
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().map(|hole| hole.into_iter().map(|p| glam::Vec3::from(p)).collect()).collect()))
        .collect();

    let mesh = Mesh::from_raw(
        vertices,
        project.mesh.indices,
        project.mesh.next_face_id,
        stored_boundaries,
        stored_holes,
    );

    Ok((mesh, project.camera))
}
