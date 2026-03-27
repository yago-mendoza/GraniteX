// Smooth shading — crease-angle normal averaging, face merging, vertex welding.
//
// Transforms imported triangle soups into properly-shaded CAD meshes:
// 1. Merge adjacent triangles with similar normals into logical faces (face_id grouping)
// 2. Average normals at shared vertices within the crease angle (smooth shading)
// 3. Weld duplicate vertices (reduce vertex count, improve GPU efficiency)

use std::collections::HashMap;
use glam::Vec3;
use super::{Mesh, Vertex};

type PosKey = (i64, i64, i64);

fn quantize_pos(p: [f32; 3]) -> PosKey {
    ((p[0] * 10000.0).round() as i64,
     (p[1] * 10000.0).round() as i64,
     (p[2] * 10000.0).round() as i64)
}

fn quantize_normal(n: [f32; 3]) -> PosKey {
    ((n[0] * 1000.0).round() as i64,
     (n[1] * 1000.0).round() as i64,
     (n[2] * 1000.0).round() as i64)
}

fn canon_edge(a: PosKey, b: PosKey) -> (PosKey, PosKey) {
    if a <= b { (a, b) } else { (b, a) }
}

struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self { parent: (0..n).collect(), rank: vec![0; n] }
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb { return; }
        if self.rank[ra] < self.rank[rb] {
            self.parent[ra] = rb;
        } else {
            self.parent[rb] = ra;
            if self.rank[ra] == self.rank[rb] {
                self.rank[ra] += 1;
            }
        }
    }
}

impl Mesh {
    /// Apply smooth shading to the mesh (typically called after import):
    ///
    /// 1. **Face merging**: Adjacent triangles with normals within `crease_angle_degrees`
    ///    are grouped into the same face_id (makes clicking select whole surfaces).
    /// 2. **Normal averaging**: Vertex normals are averaged across adjacent triangles
    ///    within the crease angle (makes cylinders/spheres look smooth).
    /// 3. **Vertex welding**: Duplicate vertices are merged (reduces vertex count ~6x).
    ///
    /// Standard crease angle: 30 degrees (same as Blender/SolidWorks default).
    pub fn apply_smooth_shading(&mut self, crease_angle_degrees: f32) {
        if self.indices.len() < 3 { return; }

        let crease_cos = crease_angle_degrees.to_radians().cos();
        let tri_count = self.indices.len() / 3;

        // --- Phase 1: Per-triangle geometric normals ---
        let tri_normals: Vec<Vec3> = self.indices.chunks_exact(3).map(|tri| {
            let p0 = Vec3::from(self.vertices[tri[0] as usize].position);
            let p1 = Vec3::from(self.vertices[tri[1] as usize].position);
            let p2 = Vec3::from(self.vertices[tri[2] as usize].position);
            (p1 - p0).cross(p2 - p0).normalize_or_zero()
        }).collect();

        // --- Phase 2: Face merging via edge adjacency + union-find ---
        let mut edge_adj: HashMap<(PosKey, PosKey), Vec<usize>> = HashMap::new();
        for (ti, tri) in self.indices.chunks_exact(3).enumerate() {
            let k = [
                quantize_pos(self.vertices[tri[0] as usize].position),
                quantize_pos(self.vertices[tri[1] as usize].position),
                quantize_pos(self.vertices[tri[2] as usize].position),
            ];
            for &(a, b) in &[(0usize, 1usize), (1, 2), (2, 0)] {
                edge_adj.entry(canon_edge(k[a], k[b])).or_default().push(ti);
            }
        }

        let mut uf = UnionFind::new(tri_count);
        for tris in edge_adj.values() {
            for i in 0..tris.len() {
                for j in (i + 1)..tris.len() {
                    let ni = tri_normals[tris[i]];
                    let nj = tri_normals[tris[j]];
                    if ni == Vec3::ZERO || nj == Vec3::ZERO { continue; }
                    if ni.dot(nj) >= crease_cos {
                        uf.union(tris[i], tris[j]);
                    }
                }
            }
        }

        // Assign sequential face_ids from union-find roots
        let mut root_to_face: HashMap<usize, u32> = HashMap::new();
        let mut next_id = 0u32;
        let tri_face_ids: Vec<u32> = (0..tri_count).map(|ti| {
            let root = uf.find(ti);
            *root_to_face.entry(root).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            })
        }).collect();

        for (ti, tri) in self.indices.chunks_exact(3).enumerate() {
            let fid = tri_face_ids[ti];
            for &vi in tri {
                self.vertices[vi as usize].face_id = fid;
            }
        }
        self.next_face_id = next_id;

        // --- Phase 3: Smooth normals ---
        // Build position -> adjacent triangles map
        let mut pos_to_tris: HashMap<PosKey, Vec<usize>> = HashMap::new();
        for (ti, tri) in self.indices.chunks_exact(3).enumerate() {
            for &vi in tri {
                let key = quantize_pos(self.vertices[vi as usize].position);
                pos_to_tris.entry(key).or_default().push(ti);
            }
        }
        for tris in pos_to_tris.values_mut() {
            tris.sort_unstable();
            tris.dedup();
        }

        // Average normals of adjacent triangles within crease angle
        for (ti, tri) in self.indices.chunks_exact(3).enumerate() {
            let my_n = tri_normals[ti];
            if my_n == Vec3::ZERO { continue; }

            for &vi in tri {
                let key = quantize_pos(self.vertices[vi as usize].position);
                let mut sum = Vec3::ZERO;
                if let Some(adj) = pos_to_tris.get(&key) {
                    for &adj_ti in adj {
                        let adj_n = tri_normals[adj_ti];
                        if adj_n != Vec3::ZERO && my_n.dot(adj_n) >= crease_cos {
                            sum += adj_n;
                        }
                    }
                }
                let result = sum.normalize_or_zero();
                if result != Vec3::ZERO {
                    self.vertices[vi as usize].normal = result.into();
                }
            }
        }

        // --- Phase 4: Vertex welding ---
        // Merge vertices with same (position, face_id, normal)
        let mut weld_map: HashMap<(PosKey, u32, PosKey), u32> = HashMap::new();
        let mut new_verts: Vec<Vertex> = Vec::new();
        let mut remap: Vec<u32> = vec![0; self.vertices.len()];

        for (old_idx, v) in self.vertices.iter().enumerate() {
            let key = (
                quantize_pos(v.position),
                v.face_id,
                quantize_normal(v.normal),
            );
            let new_idx = *weld_map.entry(key).or_insert_with(|| {
                let idx = new_verts.len() as u32;
                new_verts.push(*v);
                idx
            });
            remap[old_idx] = new_idx;
        }

        for idx in &mut self.indices {
            *idx = remap[*idx as usize];
        }
        self.vertices = new_verts;
    }
}
