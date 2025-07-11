use mlua::{Lua, Result as LuaResult};
use sysinfo::{System, SystemExt, ProcessorExt};
use std::fs;
use std::path::Path;

pub struct LuaPluginManager {
    pub lua: Lua,
    pub loaded_plugins: Vec<String>,
}

impl LuaPluginManager {
    pub fn new() -> Self {
        let lua = Lua::new();
        let mut manager = LuaPluginManager { lua, loaded_plugins: Vec::new() };
        manager.register_api();
        manager
    }

    pub fn load_plugin<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        if let Ok(code) = fs::read_to_string(&path) {
            if let Err(e) = self.lua.load(&code).exec() {
                return Err(format!("Lua error: {}", e));
            }
            self.loaded_plugins.push(path_str);
            Ok(())
        } else {
            Err("Failed to read plugin file".to_string())
        }
    }

    pub fn unload_all(&mut self) {
        // Recreate Lua state to clear all loaded scripts
        self.lua = Lua::new();
        self.register_api();
        self.loaded_plugins.clear();
    }

    pub fn load_plugins(&mut self) {
        let plugin_dir = Path::new("plugins/lua");
        if let Ok(entries) = fs::read_dir(plugin_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "lua" {
                        let _ = self.load_plugin(&path);
                    }
                }
            }
        }
    }

    pub fn register_api(&self) {
        let get_cpu = self.lua.create_function(|_, ()| {
            let mut sys = System::new_all();
            sys.refresh_cpu();
            Ok(sys.global_processor_info().cpu_usage())
        }).unwrap();
        self.lua.globals().set("get_cpu_usage", get_cpu).unwrap();
        // Add get_packet_count_by_country
        let get_packet_count_by_country = self.lua.create_function(|lua, ()| {
            let mut v = Vec::new();
            let t1 = lua.create_table()?;
            t1.set("country", "US")?;
            t1.set("count", 42)?;
            v.push(t1);
            let t2 = lua.create_table()?;
            t2.set("country", "CN")?;
            t2.set("count", 17)?;
            v.push(t2);
            Ok(v)
        }).unwrap();
        self.lua.globals().set("get_packet_count_by_country", get_packet_count_by_country).unwrap();
        // Add get_top_ports
        let get_top_ports = self.lua.create_function(|lua, n: usize| {
            let mut v = Vec::new();
            let t1 = lua.create_table()?;
            t1.set("port", 80)?;
            t1.set("count", 100)?;
            v.push(t1);
            let t2 = lua.create_table()?;
            t2.set("port", 443)?;
            t2.set("count", 80)?;
            v.push(t2);
            Ok(v)
        }).unwrap();
        self.lua.globals().set("get_top_ports", get_top_ports).unwrap();
    }
}

// API:
// get_cpu_usage() -> f32
// get_packet_count_by_country() -> Vec<(String, usize)>
// get_top_ports(n: usize) -> Vec<(u16, usize)> 