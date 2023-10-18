use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    reflect::TypePath,
    utils::{
        thiserror::{self, Error},
        BoxedFuture,
    },
};
use rive_rs::File;

#[derive(Asset, Debug, Deref, TypePath)]
pub struct Riv(pub rive_rs::File);

#[derive(Debug, Error)]
pub enum RivLoaderError {
    /// An [IO](std::io) Error.
    #[error("Could load riv: {0}")]
    Io(#[from] std::io::Error),
    /// A [RON](ron) Error
    #[error("Could not read Riv: {0}")]
    RivError(#[from] rive_rs::Error),
}

#[derive(Default)]
pub struct RivLoader;

impl AssetLoader for RivLoader {
    type Asset = Riv;
    type Settings = ();
    type Error = RivLoaderError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let file = File::new(&bytes)?;

            Ok(Riv(file))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["riv"]
    }
}
