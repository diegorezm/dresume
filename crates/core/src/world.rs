use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use typst::diag::FileResult;
use typst::foundations::{Bytes, Datetime, Dict, Duration, IntoValue};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::datetime::Time;
use typst_kit::downloader::SystemDownloader;
use typst_kit::files::{FileLoader, FileStore, FsRoot, SystemFiles};
use typst_kit::fonts::{self, FontStore};
use typst_kit::packages::SystemPackages;

/// In-memory file contents that take precedence over the disk, keyed by
/// workspace-relative path (e.g. `resumes/general.yaml`). Used for live
/// preview of unsaved edits.
pub type Overlay = HashMap<String, String>;

/// System + embedded fonts, scanned once per process.
static SYSTEM_FONTS: LazyLock<Arc<FontStore>> = LazyLock::new(|| {
    let mut store = FontStore::new();
    store.extend(fonts::system());
    store.extend(fonts::embedded());
    Arc::new(store)
});

struct Loader {
    files: SystemFiles,
    overlay: HashMap<FileId, Bytes>,
}

impl FileLoader for Loader {
    fn load(&self, id: FileId) -> FileResult<Bytes> {
        match self.overlay.get(&id) {
            Some(bytes) => Ok(bytes.clone()),
            None => self.files.load(id),
        }
    }
}

pub(crate) struct ResumeWorld {
    library: LazyHash<Library>,
    fonts: Arc<FontStore>,
    store: FileStore<Loader>,
    main: FileId,
    time: Time,
}

impl ResumeWorld {
    pub fn new(
        root: &Path,
        main: &str,
        inputs: &[(&str, &str)],
        overlay: &Overlay,
    ) -> anyhow::Result<Self> {
        let fonts = workspace_fonts(root);
        let mut dict = Dict::new();
        for (k, v) in inputs {
            dict.insert((*k).into(), (*v).into_value());
        }
        let overlay = overlay
            .iter()
            .map(|(path, text)| Ok((file_id(path)?, Bytes::from_string(text.clone()))))
            .collect::<anyhow::Result<_>>()?;
        let files = SystemFiles::new(
            FsRoot::new(root.to_path_buf()),
            SystemPackages::new(SystemDownloader::new(concat!(
                "dresume/",
                env!("CARGO_PKG_VERSION")
            ))),
        );
        Ok(Self {
            library: LazyHash::new(Library::builder().with_inputs(dict).build()),
            fonts,
            store: FileStore::new(Loader { files, overlay }),
            main: file_id(main)?,
            time: Time::system(),
        })
    }

    /// Workspace-relative path of a file, or `None` for package files.
    pub fn relative_path(&self, id: FileId) -> Option<String> {
        match id.root() {
            VirtualRoot::Project => Some(id.vpath().get_without_slash().to_string()),
            _ => None,
        }
    }
}

/// Extra fonts in `<workspace>/fonts` on top of system + embedded fonts.
fn workspace_fonts(root: &Path) -> Arc<FontStore> {
    let dir = root.join("fonts");
    if !dir.is_dir() {
        return SYSTEM_FONTS.clone();
    }
    let mut store = FontStore::new();
    store.extend(fonts::scan(&dir));
    store.extend(fonts::system());
    store.extend(fonts::embedded());
    Arc::new(store)
}

pub(crate) fn file_id(path: &str) -> anyhow::Result<FileId> {
    let vpath = VirtualPath::new(format!("/{}", path.trim_start_matches('/')))
        .map_err(|e| anyhow::anyhow!("invalid path {path}: {e:?}"))?;
    Ok(RootedPath::new(VirtualRoot::Project, vpath).intern())
}

impl World for ResumeWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.store.source(id)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.store.file(id)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        self.time.today(offset)
    }
}
