pub const ABOUT: &str = "GModPatchTool

Formerly: GModCEFCodecFix

Copyright 2020-2025, Solstice Game Studios (www.solsticegamestudios.com)
LICENSE: GNU General Public License v3.0

Purpose: Patches Garry's Mod to resolve common launch/performance issues, Updates Chromium Embedded Framework (CEF), and Enables proprietary codecs in CEF.

Guide: https://www.solsticegamestudios.com/fixmedia/
FAQ/Common Issues: https://www.solsticegamestudios.com/fixmedia/faq/
Discord: https://www.solsticegamestudios.com/discord/
Email: contact@solsticegamestudios.com\n";

use std::{
    io,
    path::{Path, PathBuf},
};

use thiserror::Error;

#[derive(Debug, Error)]
#[error("empty directory: {}", _0.display())]
struct EmptyDir(PathBuf);

trait PathExt {
    fn better_canonicalize(&self) -> io::Result<PathBuf>;
    fn is_dir_and_not_empty(&self) -> bool;
    fn to_canonical_pathbuf(&self, check_empty_directory: bool) -> io::Result<PathBuf>;
}

impl<P: AsRef<Path>> PathExt for P {
    #[inline]
    fn better_canonicalize(&self) -> io::Result<PathBuf> {
        #[cfg(windows)]
        use dunce::canonicalize;
        #[cfg(not(windows))]
        let canonicalize = Path::canonicalize;

        canonicalize(self.as_ref())
    }

    fn is_dir_and_not_empty(&self) -> bool {
        let path = self.as_ref();

        if !path.is_dir() {
            return true;
        }

        path.read_dir()
            .is_ok_and(|mut read_dir| read_dir.next().is_some())
    }

    fn to_canonical_pathbuf(&self, check_empty_directory: bool) -> io::Result<PathBuf> {
        let path = self.better_canonicalize()?;

        if !check_empty_directory || path.is_dir_and_not_empty() {
            Ok(path)
        } else {
            Err(io::Error::other(EmptyDir(path)))
        }
    }
}

trait PathBufExt {
    fn extend_and_return<P, I>(self, iter: I) -> Self
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>;
}

impl PathBufExt for PathBuf {
    #[inline]
    fn extend_and_return<P, I>(mut self, iter: I) -> Self
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>,
    {
        self.extend(iter);
        self
    }
}

pub fn get_file_hash<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let mut hasher = blake3::Hasher::new();
    hasher
        .update_mmap_rayon(path)
        .map_err(|error| error.to_string())?;
    Ok(format!("{}", hasher.finalize()))
}

#[cfg(feature = "generate")]
mod generate;

#[cfg(feature = "patch")]
mod patch;

fn main() {
    #[cfg(feature = "generate")]
    generate::main();

    #[cfg(feature = "patch")]
    patch::main();
}
