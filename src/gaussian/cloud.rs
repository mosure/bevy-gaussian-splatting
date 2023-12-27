use rand::{
    seq::SliceRandom,
    Rng,
};
use std::iter::FromIterator;

use bevy::{
    prelude::*,
    reflect::TypeUuid,
};
use serde::{
    Deserialize,
    Serialize,
};

#[cfg(feature = "sort_rayon")]
use rayon::prelude::*;

use crate::{
    gaussian::{
        f32::{
            Position,
            PositionVisibility,
            Rotation,
            ScaleOpacity,
        },
        packed::Gaussian,
    },
    material::spherical_harmonics::{
        SH_COEFF_COUNT,
        SphericalHarmonicCoefficients,
    },
};

#[cfg(feature = "f16")]
use crate::gaussian::f16::RotationScaleOpacityPacked128;


#[derive(
    Asset,
    Clone,
    Debug,
    Default,
    PartialEq,
    Reflect,
    TypeUuid,
    Serialize,
    Deserialize,
)]
#[uuid = "ac2f08eb-bc32-aabb-ff21-51571ea332d5"]
pub struct GaussianCloud {
    position_visibility: Vec<PositionVisibility>,

    spherical_harmonic: Vec<SphericalHarmonicCoefficients>,

    #[cfg(feature = "f16")]
    pub rotation_scale_opacity_packed128: Vec<RotationScaleOpacityPacked128>,

    #[cfg(not(feature = "f16"))]
    rotation: Vec<Rotation>,
    #[cfg(not(feature = "f16"))]
    scale_opacity: Vec<ScaleOpacity>,
}

impl GaussianCloud {
    pub fn is_empty(&self) -> bool {
        self.position_visibility.is_empty()
    }

    pub fn len(&self) -> usize {
        self.position_visibility.len()
    }

    pub fn position(&self, index: usize) -> &[f32; 3] {
        &self.position_visibility[index].position
    }

    pub fn position_mut(&mut self, index: usize) -> &mut [f32; 3] {
        &mut self.position_visibility[index].position
    }

    pub fn position_iter(&self) -> impl Iterator<Item = &Position> + '_ {
        self.position_visibility.iter()
            .map(|position_visibility| &position_visibility.position)
    }

    #[cfg(feature = "sort_rayon")]
    pub fn position_par_iter(&self) -> impl IndexedParallelIterator<Item = &Position> {
        self.position_visibility.par_iter()
            .map(|position_visibility| &position_visibility.position)
    }


    pub fn visibility(&self, index: usize) -> f32 {
        self.position_visibility[index].visibility
    }

    pub fn visibility_mut(&mut self, index: usize) -> &mut f32 {
        &mut self.position_visibility[index].visibility
    }


    pub fn rotation(&self, index: usize) -> &[f32; 4] {
        #[cfg(feature = "f16")]
        return &self.rotation_scale_opacity_packed128[index].rotation;

        #[cfg(not(feature = "f16"))]
        return &self.rotation[index].rotation;
    }

    pub fn rotation_mut(&mut self, index: usize) -> &mut [f32; 4] {
        #[cfg(feature = "f16")]
        return &mut self.rotation_scale_opacity_packed128[index].rotation;

        #[cfg(not(feature = "f16"))]
        return &mut self.rotation[index].rotation;
    }


    pub fn scale(&self, index: usize) -> &[f32; 3] {
        #[cfg(feature = "f16")]
        return &self.rotation_scale_opacity_packed128[index].scale;

        #[cfg(not(feature = "f16"))]
        return &self.scale_opacity[index].scale;
    }

    pub fn scale_mut(&mut self, index: usize) -> &mut [f32; 3] {
        #[cfg(feature = "f16")]
        return &mut self.rotation_scale_opacity_packed128[index].scale;

        #[cfg(not(feature = "f16"))]
        return &mut self.scale_opacity[index].scale;
    }


    pub fn gaussian(&self, index: usize) -> Gaussian {
        Gaussian {
            position_visibility: self.position_visibility[index],
            spherical_harmonic: self.spherical_harmonic[index],

            #[cfg(feature = "f16")]
            rotation_scale_opacity_packed128: self.rotation_scale_opacity_packed128[index],

            #[cfg(not(feature = "f16"))]
            rotation: self.rotation[index],
            #[cfg(not(feature = "f16"))]
            scale_opacity: self.scale_opacity[index],
        }
    }

    pub fn gaussian_iter(&self) -> impl Iterator<Item=Gaussian> + '_ {
        self.position_visibility.iter()
            .zip(self.spherical_harmonic.iter())
            .zip(self.rotation.iter())
            .zip(self.scale_opacity.iter())
            .map(|(((position_visibility, spherical_harmonic), rotation), scale_opacity)| {
                Gaussian {
                    position_visibility: *position_visibility,
                    spherical_harmonic: *spherical_harmonic,

                    #[cfg(feature = "f16")]
                    rotation_scale_opacity_packed128: *rotation_scale_opacity_packed128,

                    #[cfg(not(feature = "f16"))]
                    rotation: *rotation,
                    #[cfg(not(feature = "f16"))]
                    scale_opacity: *scale_opacity,
                }
            })
    }


    pub fn spherical_harmonic(&self, index: usize) -> &[f32; SH_COEFF_COUNT] {
        &self.spherical_harmonic[index].coefficients
    }

    pub fn spherical_harmonic_mut(&mut self, index: usize) -> &mut [f32; SH_COEFF_COUNT] {
        &mut self.spherical_harmonic[index].coefficients
    }
}


impl GaussianCloud {
    pub fn subset(&self, indicies: &[usize]) -> Self {
        let mut position_visibility = Vec::with_capacity(indicies.len());
        let mut spherical_harmonic = Vec::with_capacity(indicies.len());
        let mut rotation = Vec::with_capacity(indicies.len());
        let mut scale_opacity = Vec::with_capacity(indicies.len());

        for &index in indicies.iter() {
            position_visibility.push(self.position_visibility[index]);
            spherical_harmonic.push(self.spherical_harmonic[index]);

            #[cfg(feature = "f16")]
            rotation_scale_opacity_packed128.push(self.rotation_scale_opacity_packed128[index]);

            #[cfg(not(feature = "f16"))]
            rotation.push(self.rotation[index]);
            #[cfg(not(feature = "f16"))]
            scale_opacity.push(self.scale_opacity[index]);
        }

        Self {
            position_visibility,
            spherical_harmonic,

            #[cfg(feature = "f16")]
            rotation_scale_opacity_packed128,

            #[cfg(not(feature = "f16"))]
            rotation,
            #[cfg(not(feature = "f16"))]
            scale_opacity,
        }
    }

    pub fn to_packed(&self) -> Vec<Gaussian> {
        let mut gaussians = Vec::with_capacity(self.len());

        for index in 0..self.len() {
            gaussians.push(self.gaussian(index));
        }

        gaussians
    }
}


impl GaussianCloud {
    pub fn from_gaussians(gaussians: Vec<Gaussian>) -> Self {
        let mut position_visibility = Vec::with_capacity(gaussians.len());
        let mut spherical_harmonic = Vec::with_capacity(gaussians.len());
        let mut rotation = Vec::with_capacity(gaussians.len());
        let mut scale_opacity = Vec::with_capacity(gaussians.len());

        for gaussian in gaussians {
            position_visibility.push(gaussian.position_visibility);
            spherical_harmonic.push(gaussian.spherical_harmonic);

            #[cfg(feature = "f16")]
            rotation_scale_opacity_packed128.push(gaussian.rotation_scale_opacity_packed128);

            #[cfg(not(feature = "f16"))]
            rotation.push(gaussian.rotation);
            #[cfg(not(feature = "f16"))]
            scale_opacity.push(gaussian.scale_opacity);
        }

        Self {
            position_visibility,
            spherical_harmonic,

            #[cfg(feature = "f16")]
            rotation_scale_opacity_packed128,

            #[cfg(not(feature = "f16"))]
            rotation,
            #[cfg(not(feature = "f16"))]
            scale_opacity,
        }
    }

    pub fn test_model() -> Self {
        let origin = Gaussian {
            rotation: [
                1.0,
                0.0,
                0.0,
                0.0,
            ].into(),
            position_visibility: [
                0.0,
                0.0,
                0.0,
                1.0,
            ].into(),
            scale_opacity: [
                0.5,
                0.5,
                0.5,
                0.5,
            ].into(),
            spherical_harmonic: SphericalHarmonicCoefficients {
                coefficients: {
                    let mut rng = rand::thread_rng();
                    let mut coefficients = [0.0; SH_COEFF_COUNT];

                    for coefficient in coefficients.iter_mut() {
                        *coefficient = rng.gen_range(-1.0..1.0);
                    }

                    coefficients
                },
            },
        };
        let mut gaussians: Vec<Gaussian> = Vec::new();

        for &x in [-0.5, 0.5].iter() {
            for &y in [-0.5, 0.5].iter() {
                for &z in [-0.5, 0.5].iter() {
                    let mut g = origin;
                    g.position_visibility = [x, y, z, 1.0].into();
                    gaussians.push(g);

                    let mut rng = rand::thread_rng();
                    gaussians.last_mut().unwrap().spherical_harmonic.coefficients.shuffle(&mut rng);
                }
            }
        }

        gaussians.push(gaussians[0]);

        GaussianCloud::from_gaussians(gaussians)
    }
}

impl FromIterator<Gaussian> for GaussianCloud {
    fn from_iter<I: IntoIterator<Item=Gaussian>>(iter: I) -> Self {
        let gaussians = iter.into_iter().collect::<Vec<Gaussian>>();
        GaussianCloud::from_gaussians(gaussians)
    }
}
