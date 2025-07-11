-- Minimal SysPort Lua plugin example
function on_load()
    print("Sample plugin loaded!")
end

function on_unload()
    print("Sample plugin unloaded!")
end

function hello()
    print("Hello from the plugin!")
end

register_command("hello", hello) 