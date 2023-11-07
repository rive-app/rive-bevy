![CI Rive Rust](https://github.com/rive-app/rive-rs/actions/workflows/ci.yml/badge.svg)
![Discord badge](https://img.shields.io/discord/532365473602600965)
![Twitter handle](https://img.shields.io/twitter/follow/rive_app.svg?style=social&label=Follow)

# rive-bevy

![Rive hero image](https://cdn.rive.app/rive_logo_dark_bg.png)

A Bevy runtime library for [Rive](https://rive.app).

> [!NOTE]  
> This runtime uses [Vello](https://github.com/linebender/vello) as a render back-end, which has certain limitations. Refer to [Known Issues](#known-issues) for details. Efforts are underway to incorporate the [Rive Renderer](https://rive.app/renderer) as another back-end.

## Table of contents

- ⭐️ [Rive Overview](#rive-overview)
- 🚀 [Getting Started](#getting-started)
- 👨‍💻 [Contributing](#contributing)
- ❓ [Issues](#issues)

## Rive Overview

[Rive](https://rive.app) is a real-time interactive design and animation tool that helps teams
create and run interactive animations anywhere. Designers and developers use our collaborative
editor to create motion graphics that respond to different states and user inputs. Our lightweight
open-source runtime libraries allow them to load their animations into apps, games, and websites.

🏡 [Homepage](https://rive.app/)

📘 [General help docs](https://help.rive.app/)

🛠 [Learning Rive](https://rive.app/learn-rive/)

## Getting Started

The Rive Bevy runtime makes use of the [Rive Rust runtime](https://github.com/rive-app/rive-rs).

You will need a Rust toolchain and a C compiler to build. You can can install
the Rust toolchain using [rustup].

Run one of the example projects:

```bash
git clone https://github.com/rive-app/rive-bevy
cd rive-bevy/
cargo run --example ui-on-cube
```

There are a number of demos/games in the examples folder that showcase various Rive features.

See the [Rive Bevy documentation](https://help.rive.app/game-frameworks/bevy) for additional guides.

### Awesome Rive

For even more examples and resources on using Rive at runtime or in other tools, checkout the [awesome-rive](https://github.com/rive-app/awesome-rive) repo.

## Contributing

We love contributions!

If you need to make changes to the underlying [Rive Rust runtime](https://github.com/rive-app/rive-rs) code you'll need to update your dependencies to point to a local version of the package.

```TOML
rive-rs = { path = "/loca/path/to/rive-rs", features = [
    "vello",
] }
```

## Issues

Have an issue with using the runtime, or want to suggest a feature/API to help make your development
life better? Log an issue in our [issues](https://github.com/rive-app/rive-bevy/issues) tab! You
can also browse older issues and discussion threads there to see solutions that may have worked for
common problems.

### Known Issues

The existing [Vello](https://github.com/linebender/vello) render back-end may lead to some inconsistencies in comparison to the original design:

- Image meshes: There could be gaps between triangles if the mesh is transparent, and it may render incorrectly with a large number of animations.
- All strokes will have round joins and caps.

Efforts are being made to make the [Rive Renderer](https://rive.app/renderer) available. You'll then have the choice to select your preferred renderer.