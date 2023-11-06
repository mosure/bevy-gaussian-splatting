use rand::seq::SliceRandom;
use std::{
    io::{
        BufReader,
        Cursor,
        ErrorKind,
    },
    marker::Copy,
};

use bevy::{
    prelude::*,
    asset::{
        AssetLoader,
        AsyncReadExt,
        LoadContext,
        io::Reader,
    },
    reflect::TypeUuid,
    render::render_resource::ShaderType,
    utils::BoxedFuture,
};
use bincode2::deserialize_from;
use bytemuck::{
    Pod,
    Zeroable,
};
use flate2::read::GzDecoder;
use serde::{
    Deserialize,
    Serialize,
    Serializer,
    ser::SerializeTuple,
};

use crate::ply::parse_ply;


const fn num_sh_coefficients(degree: usize) -> usize {
    if degree == 0 {
        1
    } else {
        2 * degree + 1 + num_sh_coefficients(degree - 1)
    }
}
const SH_DEGREE: usize = 3;
pub const SH_CHANNELS: usize = 3;
pub const MAX_SH_COEFF_COUNT_PER_CHANNEL: usize = num_sh_coefficients(SH_DEGREE);
pub const MAX_SH_COEFF_COUNT: usize = MAX_SH_COEFF_COUNT_PER_CHANNEL * SH_CHANNELS;
#[derive(
    Clone,
    Copy,
    Debug,
    Reflect,
    ShaderType,
    Pod,
    Zeroable,
    Serialize,
    Deserialize,
)]
#[repr(C)]
pub struct SphericalHarmonicCoefficients {
    #[serde(serialize_with = "coefficients_serializer", deserialize_with = "coefficients_deserializer")]
    pub coefficients: [f32; MAX_SH_COEFF_COUNT],
}
impl Default for SphericalHarmonicCoefficients {
    fn default() -> Self {
        Self {
            coefficients: [0.0; MAX_SH_COEFF_COUNT],
        }
    }
}
fn coefficients_serializer<S>(n: &[f32; MAX_SH_COEFF_COUNT], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut tup = s.serialize_tuple(MAX_SH_COEFF_COUNT)?;
    for &x in n.iter() {
        tup.serialize_element(&x)?;
    }

    tup.end()
}

fn coefficients_deserializer<'de, D>(d: D) -> Result<[f32; MAX_SH_COEFF_COUNT], D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct CoefficientsVisitor;

    impl<'de> serde::de::Visitor<'de> for CoefficientsVisitor {
        type Value = [f32; MAX_SH_COEFF_COUNT];

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an array of floats")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<[f32; MAX_SH_COEFF_COUNT], A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut coefficients = [0.0; MAX_SH_COEFF_COUNT];

            for (i, coefficient) in coefficients.iter_mut().enumerate().take(MAX_SH_COEFF_COUNT) {
                *coefficient = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
            }
            Ok(coefficients)
        }
    }

    d.deserialize_tuple(MAX_SH_COEFF_COUNT, CoefficientsVisitor)
}


pub const MAX_SIZE_VARIANCE: f32 = 5.0;

#[derive(
    Clone,
    Debug,
    Default,
    Copy,
    Reflect,
    ShaderType,
    Pod,
    Zeroable,
    Serialize,
    Deserialize,
)]
#[repr(C)]
// TODO: support f16 gaussian clouds (shader and asset loader)
pub struct Gaussian {
    pub rotation: [f32; 4],
    pub position: [f32; 4],
    pub scale_opacity: [f32; 4],
    pub spherical_harmonic: SphericalHarmonicCoefficients,
}

#[derive(
    Asset,
    Clone,
    Debug,
    Reflect,
    TypeUuid,
    Serialize,
    Deserialize,
)]
#[uuid = "ac2f08eb-bc32-aabb-ff21-51571ea332d5"]
pub struct GaussianCloud(pub Vec<Gaussian>);

impl GaussianCloud {
    pub fn test_model() -> Self {
        let origin = Gaussian {
            rotation: [
                1.0,
                0.0,
                0.0,
                0.0,
            ],
            position: [
                0.0,
                0.0,
                0.0,
                1.0,
            ],
            scale_opacity: [
                0.5,
                0.5,
                0.5,
                0.5,
            ],
            spherical_harmonic: SphericalHarmonicCoefficients{
                coefficients: [
                    1.0, 0.0, 1.0,
                    0.0, 0.5, 0.0,
                    0.3, 0.2, 0.0,
                    0.4, 0.0, 0.2,
                    0.1, 0.0, 0.0,
                    0.0, 0.3, 0.3,
                    0.0, 1.0, 1.0,
                    0.3, 0.0, 0.0,
                    0.0, 0.0, 0.0,
                    0.0, 0.3, 1.0,
                    0.5, 0.3, 0.0,
                    0.2, 0.3, 0.1,
                    0.6, 0.3, 0.1,
                    0.0, 0.3, 0.2,
                    0.0, 0.5, 0.3,
                    0.6, 0.1, 0.2,
                ],
            },
        };
        let mut cloud = GaussianCloud(Vec::new());

        for &x in [-0.5, 0.5].iter() {
            for &y in [-0.5, 0.5].iter() {
                for &z in [-0.5, 0.5].iter() {
                    let mut g = origin;
                    g.position = [x, y, z, 1.0];
                    cloud.0.push(g);

                    let mut rng = rand::thread_rng();
                    cloud.0.last_mut().unwrap().spherical_harmonic.coefficients.shuffle(&mut rng);
                }
            }
        }

        cloud
    }
}


#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct GaussianCloudSettings {
    pub aabb: bool,
    pub global_scale: f32,
    pub global_transform: GlobalTransform,
    pub visualize_bounding_box: bool,
}

impl Default for GaussianCloudSettings {
    fn default() -> Self {
        Self {
            aabb: false,
            global_scale: 1.0,
            global_transform: Transform::IDENTITY.into(),
            visualize_bounding_box: false,
        }
    }
}


#[derive(Default)]
pub struct GaussianCloudLoader;

impl AssetLoader for GaussianCloudLoader {
    type Asset = GaussianCloud;
    type Settings = ();
    type Error = std::io::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {

        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            match load_context.path().extension() {
                Some(ext) if ext == "ply" => {
                    let cursor = Cursor::new(bytes);
                    let mut f = BufReader::new(cursor);

                    let ply_cloud = parse_ply(&mut f)?;
                    let cloud = GaussianCloud(ply_cloud);

                    Ok(cloud)
                },
                Some(ext) if ext == "gcloud" => {
                    let decompressed = GzDecoder::new(bytes.as_slice());
                    let cloud: GaussianCloud = deserialize_from(decompressed).expect("failed to decode cloud");

                    Ok(cloud)
                },
                _ => Err(std::io::Error::new(ErrorKind::Other, "only .ply and .gcloud supported")),
            }
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ply", "gcloud"]
    }
}
