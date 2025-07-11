use mlua::{Lua, Result as LuaResult};
use sysinfo::{System, SystemExt, ProcessorExt};

#[no_mangle]
pub extern "C" fn plugin_entry() {
    let lua = Lua::new();
    let mut sys = System::new_all();
    sys.refresh_cpu();
    let cpu = sys.global_processor_info().cpu_usage();
    println!("[sample_plugin] CPU usage: {}%", cpu);
    let _ = lua.load(r#"print('Hello from sample_plugin!')"#).exec();
}
