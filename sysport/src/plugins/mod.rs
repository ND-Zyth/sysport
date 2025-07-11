use std::fs;
use std::path::{Path, PathBuf};
use libloading::{Library, Symbol};

pub struct LoadedPlugin {
    pub name: String,
    pub lib: Library,
}

pub struct PluginSystem {
    pub loaded_plugins: Vec<LoadedPlugin>,
}

impl PluginSystem {
    pub fn new() -> Self {
        Self { loaded_plugins: Vec::new() }
    }

    pub fn load_plugins(&mut self, plugins_dir: &str) {
        let ext = std::env::consts::DLL_EXTENSION;
        let dir = Path::new(plugins_dir);
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(e) = path.extension() {
                    if e == ext {
                        let name = path.file_stem().unwrap().to_string_lossy().to_string();
                        unsafe {
                            match Library::new(&path) {
                                Ok(lib) => {
                                    let plugin_entry: Symbol<unsafe extern "C" fn()> =
                                        lib.get(b"plugin_entry").expect("plugin_entry not found");
                                    plugin_entry();
                                    self.loaded_plugins.push(LoadedPlugin { name, lib });
                                }
                                Err(e) => {
                                    eprintln!("Failed to load plugin {}: {}", path.display(), e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
} 