use bevy::{
    prelude::*,
    asset::LoadState,
};
use kd_tree::{
    KdPoint,
    KdTree,
};
use typenum::consts::U3;

use crate::{
    Gaussian,
    GaussianCloud,
    query::select::Select,
};


#[derive(Component, Debug, Reflect)]
pub struct SparseSelect {
    pub radius: f32,
    pub neighbor_threshold: usize,
    pub completed: bool,
}

impl Default for SparseSelect {
    fn default() -> Self {
        Self {
            radius: 0.05,
            neighbor_threshold: 4,
            completed: false,
        }
    }
}


#[derive(Default)]
pub struct SparsePlugin;

impl Plugin for SparsePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SparseSelect>();

        app.add_systems(Update, select_sparse);
    }
}


impl KdPoint for Gaussian {
    type Scalar = f32;
    type Dim = U3;

    fn at(&self, i: usize) -> Self::Scalar {
        self.position_visibility[i]
    }
}


fn select_sparse(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    gaussian_clouds_res: Res<Assets<GaussianCloud>>,
    mut selections: Query<(
        Entity,
        &Handle<GaussianCloud>,
        &mut SparseSelect,
    )>,
) {
    for (
        entity,
        cloud_handle,
        mut select,
    ) in selections.iter_mut() {
        if Some(LoadState::Loading) == asset_server.get_load_state(cloud_handle) {
            continue;
        }

        if Some(LoadState::Loading) == asset_server.get_load_state(cloud_handle) {
            continue;
        }

        if select.completed {
            continue;
        }
        select.completed = true;

        let cloud = gaussian_clouds_res.get(cloud_handle).unwrap();
        let tree = KdTree::build_by_ordered_float(cloud.gaussians.clone());

        let new_selection = cloud.gaussians.iter()
            .enumerate()
            .filter(|(_idx, gaussian)| {
                let neighbors = tree.within_radius(*gaussian, select.radius);

                neighbors.len() < select.neighbor_threshold
            })
            .map(|(idx, _gaussian)| idx)
            .collect::<Select>();

        commands.entity(entity)
            .remove::<Select>()
            .insert(new_selection);
    }
}