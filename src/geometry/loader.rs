use std::{fs::File, io::{BufReader, BufRead}};
use bytemuck::{Zeroable, Pod};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct BasicVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct UnlitVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

struct RawVertex(f32, f32, f32);

impl RawVertex {
    /// Loads vertex data from a str containing 3 values separated by whitespace, for example `0.0 0.0 0.0`
    fn from_str(input: &str) -> RawVertex {
        let mut contents: Vec<f32> = input.split_whitespace()
            .map(|item| item.parse().expect(&format!("Unable to parse element \"{}\"", input)))
            .collect();
        if contents.len() == 2 {
            contents.push(0.0);
        }
        RawVertex (contents[0], contents[1], contents[2])
    }

    fn to_arr(&self) -> [f32; 3] {
        [self.0, self.1, self.2]
    }
}

#[derive(Debug)]
pub struct RawFace {
    pub vertex_indices: [usize; 3],
    pub normal_indices: Option<[usize; 3]>,
    pub tex_coord_indices: Option<[usize; 3]>,
}

impl RawFace {
    pub fn from_str(input: &str, invert: bool) -> Self {
        let args = RawFace::parse_args(input);
        Self {
            vertex_indices: RawFace::get_indices(&args, 0, invert).unwrap(),
            tex_coord_indices: RawFace::get_indices(&args, 1, invert),
            normal_indices: RawFace::get_indices(&args, 2, invert),

        }
    }
    /// Parses an vertex of the face declaration in the format `v/vt/vn v/vt/vn v/vt/vn`, returning a value in the format
    /// `[[v, vt, vn], [v, vt, vn], [v, vt, vn]]`
    fn parse_args(input: &str) -> Vec<Vec<Option<usize>>> {
        input.split_whitespace()
            .map(|item| {
                let mut contents: Vec<Option<usize>> = item.split('/')
                    .map(|item| item.parse().ok())
                    .collect();
                while contents.len() < 3 {
                    contents.push(None);
                }
                contents
            })
            .collect::<Vec<Vec<Option<usize>>>>()
    }

    /// Gets the indices of a specified type for all of the vertices in the face (e.g., all of the normal indices), returning None
    /// if any of the indices are None for that data type
    fn get_indices(input: &Vec<Vec<Option<usize>>>, data_type_idx: usize, invert: bool) -> Option<[usize; 3]> {
        let index_iter = [
            input[0][data_type_idx],
            input[1][data_type_idx],
            input[2][data_type_idx],
        ].into_iter().map(|wrapped_idx| wrapped_idx.map(|idx| idx - 1));

        if invert { 
            return index_iter.rev()
                .collect::<Option<Vec<usize>>>()
                .map(|val| val.try_into().unwrap())
        } else {
            return index_iter
                .collect::<Option<Vec<usize>>>()
                .map(|val| val.try_into().unwrap())
        }
    }
}


pub struct ModelBuilder {
    name: Option<String>,
    vertices: Vec<RawVertex>,
    normals: Vec<RawVertex>,
    tex_coords: Vec<RawVertex>,
    faces: Vec<RawFace>,
}

impl ModelBuilder {
    pub fn from_file(filename: &'static str, invert_winding_order: bool) -> Self {
        let data = File::open(filename).unwrap();
        let buffered_data = BufReader::new(data);

        let mut name = None;

        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut tex_coords = Vec::new();
        let mut faces = Vec::new();

        for line in buffered_data.lines() {
            let line = line.unwrap();
            match line.split_at(2) {
                ("o ", val) => { name = Some(String::from(val)); }
                ("v ", val) => { vertices.push(RawVertex::from_str(val)) }
                ("vn", val) => { normals.push(RawVertex::from_str(val)) }
                ("vt", val) => { tex_coords.push(RawVertex::from_str(val)) }
                ("f ", val) => { faces.push(RawFace::from_str(val, invert_winding_order)) }
                (_, _) => {}
            }
        }

        Self {
            name,
            vertices,
            normals,
            tex_coords,
            faces,
        }
    }

    /// Builds an array of vertices from the model. Does not require texture coordinates in the loaded model, but does
    /// require normals and vertices.
    pub fn build_basic(&self, custom_color: [f32; 3]) -> Vec<BasicVertex> {
        let mut result = Vec::new();
        for face in &self.faces {
            let verts = face.vertex_indices;
            let norms = face.normal_indices.unwrap();
            for i in 0..3 {
                result.push(BasicVertex{
                    position: self.vertices[verts[i]].to_arr(),
                    normal: self.normals[norms[i]].to_arr(),
                    color: custom_color,
                });
            }
        };
        result
    }

    /// Builds an array of unlit vertices from the model. Does not require texture coordinates or normals in the loaded model, but 
    /// does require vertices.
    pub fn build_unlit(&self, custom_color: [f32; 3]) -> Vec<UnlitVertex> {
        let mut result = Vec::new();
        for face in &self.faces {
            let verts = face.vertex_indices;
            for i in 0..3 {
                result.push(UnlitVertex{
                    position: self.vertices[verts[i]].to_arr(),
                    color: custom_color,
                })
            }
        };
        result
    }
}

