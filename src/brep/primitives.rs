// BREP primitive constructors.

use glam::Vec3;
use super::{BrepMesh, FaceId};

impl BrepMesh {
    /// Unit cube centered at origin (±0.5).
    pub fn cube() -> Self {
        let mut m = BrepMesh::new();

        // 8 vertices of a cube
        let v = [
            m.add_vertex(Vec3::new(-0.5, -0.5,  0.5)), // 0: front-bottom-left
            m.add_vertex(Vec3::new( 0.5, -0.5,  0.5)), // 1: front-bottom-right
            m.add_vertex(Vec3::new( 0.5,  0.5,  0.5)), // 2: front-top-right
            m.add_vertex(Vec3::new(-0.5,  0.5,  0.5)), // 3: front-top-left
            m.add_vertex(Vec3::new(-0.5, -0.5, -0.5)), // 4: back-bottom-left
            m.add_vertex(Vec3::new( 0.5, -0.5, -0.5)), // 5: back-bottom-right
            m.add_vertex(Vec3::new( 0.5,  0.5, -0.5)), // 6: back-top-right
            m.add_vertex(Vec3::new(-0.5,  0.5, -0.5)), // 7: back-top-left
        ];

        // 6 faces (CCW winding when viewed from outside)
        m.add_face(&[v[0], v[1], v[2], v[3]], Vec3::Z);        // front (+Z)
        m.add_face(&[v[5], v[4], v[7], v[6]], Vec3::NEG_Z);    // back (-Z)
        m.add_face(&[v[3], v[2], v[6], v[7]], Vec3::Y);        // top (+Y)
        m.add_face(&[v[4], v[5], v[1], v[0]], Vec3::NEG_Y);    // bottom (-Y)
        m.add_face(&[v[1], v[5], v[6], v[2]], Vec3::X);        // right (+X)
        m.add_face(&[v[4], v[0], v[3], v[7]], Vec3::NEG_X);    // left (-X)

        m
    }
}
