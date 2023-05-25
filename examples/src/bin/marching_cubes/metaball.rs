use std::sync::Arc;

use rhyolite::geometry::marched::Metaball;
use rhyolite::renderer::marched::to_partially_init_arr;
use rhyolite::renderer::staging::UniformSrc;
use vulkano::descriptor_set::layout::DescriptorSetLayout;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::padded::Padded;
use rhyolite::renderer::mesh::MeshRenderer;
use rhyolite::shaders::marched_frag;

const MAX_METABALLS: usize = 1024;

pub fn metaball_set(renderer: &MeshRenderer, objects: &Vec<Metaball>, layout: Arc<DescriptorSetLayout>) -> Arc<PersistentDescriptorSet> {
    let objects: Vec<Padded<marched_frag::UMetaball, 12>> = objects
        .iter()
        .map(|obj| {
            Padded::from(obj.get_raw())
        })
        .collect();

    let len = objects.len() as u32;
    let data = unsafe {
        to_partially_init_arr::<MAX_METABALLS, Padded<marched_frag::UMetaball, 12>>(objects)
    };

    let metaball_buf = renderer.get_subbuffer_allocator().allocate_unsized(MAX_METABALLS as u64).unwrap();
    *metaball_buf.write().unwrap() = marched_frag::UMetaballData { data, len };

    PersistentDescriptorSet::new(
        &renderer.get_descriptor_set_allocator(),
        layout.clone(),
        [WriteDescriptorSet::buffer(0, metaball_buf.clone())],
    ).expect("Unable to create geometry descriptor set")
}
