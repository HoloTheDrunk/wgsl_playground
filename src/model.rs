use texture::Texture;
use wgpu::util::RenderEncoder;

use super::*;
use std::{
    io::{BufReader, Cursor},
    mem,
    ops::Range,
};

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl Vertex for ModelVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

pub struct Material {
    pub name: String,
    pub bind_group: wgpu::BindGroup,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

impl Model {
    pub async fn from_file(
        asset_path: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<model::Model> {
        let obj_text = {
            let path = std::path::Path::new(env!("OUT_DIR"))
                .join("assets")
                .join(asset_path);
            std::fs::read_to_string(path)?
        };
        let obj_cursor = Cursor::new(obj_text);
        let mut obj_reader = BufReader::new(obj_cursor);

        let (models, obj_materials) = tobj::load_obj_buf_async(
            &mut obj_reader,
            &tobj::LoadOptions {
                single_index: true,
                triangulate: true,
                ..Default::default()
            },
            |p| async move {
                let mat_text = {
                    let path = std::path::Path::new(env!("OUT_DIR")).join("assets").join(p);
                    std::fs::read_to_string(path.clone()).unwrap()
                };
                dbg!(tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(
                    mat_text
                ))))
            },
        )
        .await?;

        let mut materials = Vec::new();
        for m in obj_materials? {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("{} Bind Group", m.name)),
                layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(&format!("{} Bind Group Layout", m.name)),
                    entries: &[],
                }),
                entries: &[],
            });

            materials.push(model::Material {
                name: m.name,
                bind_group,
            });
        }

        let meshes = models
            .into_iter()
            .map(|m| {
                let vertices = (0..m.mesh.positions.len() / 3)
                    .map(|i| {
                        if m.mesh.normals.is_empty() {
                            model::ModelVertex {
                                position: [
                                    m.mesh.positions[i * 3],
                                    m.mesh.positions[i * 3 + 1],
                                    m.mesh.positions[i * 3 + 2],
                                ],
                                tex_coords: [
                                    m.mesh.texcoords[i * 2],
                                    1.0 - m.mesh.texcoords[i * 2 + 1],
                                ],
                                normal: [0.0, 0.0, 0.0],
                            }
                        } else {
                            model::ModelVertex {
                                position: [
                                    m.mesh.positions[i * 3],
                                    m.mesh.positions[i * 3 + 1],
                                    m.mesh.positions[i * 3 + 2],
                                ],
                                tex_coords: [
                                    m.mesh.texcoords[i * 2],
                                    1.0 - m.mesh.texcoords[i * 2 + 1],
                                ],
                                normal: [
                                    m.mesh.normals[i * 3],
                                    m.mesh.normals[i * 3 + 1],
                                    m.mesh.normals[i * 3 + 2],
                                ],
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{} Vertex Buffer", m.name)),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{} Index Buffer", m.name)),
                    contents: bytemuck::cast_slice(&m.mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                Mesh {
                    name: asset_path.to_owned(),
                    vertex_buffer,
                    index_buffer,
                    num_elements: m.mesh.indices.len() as u32,
                    material: m.mesh.material_id.unwrap_or(0),
                }
            })
            .collect::<Vec<_>>();

        Ok(Self { meshes, materials })
    }
}

pub trait DrawModel<'model> {
    fn draw_mesh(
        &mut self,
        mesh: &'model Mesh,
        material: &'model Material,
        camera_bind_group: &'model wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'model Mesh,
        material: &'model Material,
        instances: Range<u32>,
        camera_bind_group: &'model wgpu::BindGroup,
    );

    fn draw_model(&mut self, model: &'model Model, camera_bind_group: &'model wgpu::BindGroup);
    fn draw_model_instanced(
        &mut self,
        model: &'model Model,
        instances: Range<u32>,
        camera_bind_group: &'model wgpu::BindGroup,
    );
}

impl<'a, 'model> DrawModel<'model> for wgpu::RenderPass<'a>
where
    'model: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'model Mesh,
        material: &'model Material,
        camera_bind_group: &'model wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, camera_bind_group);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'model Mesh,
        material: &'model Material,
        instances: Range<u32>,
        camera_bind_group: &'model wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(&mut self, model: &'model Model, camera_bind_group: &'model wgpu::BindGroup) {
        self.draw_model_instanced(model, 0..1, camera_bind_group);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'model Model,
        instances: Range<u32>,
        camera_bind_group: &'model wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(mesh, material, instances.clone(), camera_bind_group);
        }
    }
}
