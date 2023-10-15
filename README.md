# bevy_gaussian_splatting 🌌

[![test](https://github.com/mosure/bevy_gaussian_splatting/workflows/test/badge.svg)](https://github.com/Mosure/bevy_gaussian_splatting/actions?query=workflow%3Atest)
[![GitHub License](https://img.shields.io/github/license/mosure/bevy_gaussian_splatting)](https://raw.githubusercontent.com/mosure/bevy_gaussian_splatting/main/LICENSE)
[![GitHub Last Commit](https://img.shields.io/github/last-commit/mosure/bevy_gaussian_splatting)](https://github.com/mosure/bevy_gaussian_splatting)
[![GitHub Releases](https://img.shields.io/github/v/release/mosure/bevy_gaussian_splatting?include_prereleases&sort=semver)](https://github.com/mosure/bevy_gaussian_splatting/releases)
[![GitHub Issues](https://img.shields.io/github/issues/mosure/bevy_gaussian_splatting)](https://github.com/mosure/bevy_gaussian_splatting/issues)
[![Average time to resolve an issue](https://isitmaintained.com/badge/resolution/mosure/bevy_gaussian_splatting.svg)](http://isitmaintained.com/project/mosure/bevy_gaussian_splatting)
[![crates.io](https://img.shields.io/crates/v/bevy_gaussian_splatting.svg)](https://crates.io/crates/bevy_gaussian_splatting)

![Alt text](docs/notferris.png)

bevy gaussian splatting render pipeline plugin

`cargo run -- {path to ply or gcloud.gz file}`

## capabilities

- [X] ply to gcloud converter
- [X] gcloud and ply asset loaders
- [X] bevy gaussian cloud render pipeline
- [ ] 4D gaussian clouds via morph targets
- [ ] bevy 3D camera to gaussian cloud pipeline

## usage

```rust
use bevy::prelude::*;
use bevy_gaussian_splatting::GaussianSplattingPlugin;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugins(GaussianSplattingPlugin)
        .add_systems(Startup, setup_gaussian_cloud)
        .run();
}

fn setup_gaussian_cloud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(GaussianSplattingBundle {
        cloud: asset_server.load("scenes/icecream.ply"),
        ..Default::default()
    });

    commands.spawn(Camera3dBundle::default());
}
```


## compatible bevy versions

| `bevy_gaussian_splatting` | `bevy` |
| :--           | :--    |
| `0.1`         | `0.11` |


# credits

- [4d gaussians](https://github.com/hustvl/4DGaussians)
- [bevy](https://github.com/bevyengine/bevy)
- [bevy-hanabi](https://github.com/djeedai/bevy_hanabi)
- [diff-gaussian-rasterization](https://github.com/graphdeco-inria/diff-gaussian-rasterization)
- [dreamgaussian](https://github.com/dreamgaussian/dreamgaussian)
- [dynamic-3d-gaussians](https://github.com/JonathonLuiten/Dynamic3DGaussians)
- [ewa splatting](https://www.cs.umd.edu/~zwicker/publications/EWASplatting-TVCG02.pdf)
- [gaussian-splatting](https://github.com/graphdeco-inria/gaussian-splatting)
- [gaussian-splatting-web](https://github.com/cvlab-epfl/gaussian-splatting-web)
- [making gaussian splats smaller](https://aras-p.info/blog/2023/09/13/Making-Gaussian-Splats-smaller/)
- [onesweep](https://arxiv.org/ftp/arxiv/papers/2206/2206.01784.pdf)
- [point-visualizer](https://github.com/mosure/point-visualizer)
- [rusty-automata](https://github.com/mosure/rusty-automata)
- [splat](https://github.com/antimatter15/splat)
- [splatter](https://github.com/Lichtso/splatter)
- [sturdy-dollop](https://github.com/mosure/sturdy-dollop)
- [taichi_3d_gaussian_splatting](https://github.com/wanmeihuali/taichi_3d_gaussian_splatting)
