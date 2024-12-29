#[allow(unused_imports)]
use std::io::{
    BufReader,
    Cursor,
    ErrorKind,
};

use bevy::asset::{
    AssetLoader,
    LoadContext,
    io::Reader,
};

use crate::{
    io::codec::CloudCodec,
    gaussian::packed::PlanarGaussian3d,
};


#[derive(Default)]
pub struct Gaussian3dLoader;

impl AssetLoader for Gaussian3dLoader {
    type Asset = PlanarGaussian3d;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        match load_context.path().extension() {
            Some(ext) if ext == "ply" => {
                #[cfg(feature = "io_ply")]
                {
                    let cursor = Cursor::new(bytes);
                    let mut f = BufReader::new(cursor);

                    Ok(crate::io::ply::parse_ply(&mut f)?)
                }

                #[cfg(not(feature = "io_ply"))]
                {
                    Err(std::io::Error::new(ErrorKind::Other, "ply support not enabled, enable with io_ply feature"))
                }
            },
            Some(ext) if ext == "gcloud" => {
                let cloud = PlanarGaussian3d::decode(bytes.as_slice());

                Ok(cloud)
            },
            _ => Err(std::io::Error::new(ErrorKind::Other, "only .ply and .gcloud supported")),
        }
    }

    fn extensions(&self) -> &[&str] {
        &["ply", "gcloud"]
    }
}
