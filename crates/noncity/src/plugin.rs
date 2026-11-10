use rhai::{Engine, Scope, AST};
use std::path::Path;
use tracing::{error, info};

pub struct PluginManager {
    engine: Engine,
    scripts: Vec<(String, AST)>,
}

impl PluginManager {
    pub fn new<P: AsRef<Path>>(plugin_dir: P) -> Self {
        let mut engine = Engine::new();
        engine.set_max_expr_depths(64, 32);

        let dir = plugin_dir.as_ref();
        if !dir.exists() {
            let _ = std::fs::create_dir_all(dir);
            let default_script = r#"
fn on_player_login(player_name, ip) {
    return true;
}

fn on_command(player_name, command) {
    if command == "ping" {
        return "Pong from Rhai plugin!";
    }
    return "";
}
"#;
            let _ = std::fs::write(dir.join("main.rhai"), default_script.trim_start());
        }

        let mut scripts = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("rhai") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        match engine.compile(&content) {
                            Ok(ast) => {
                                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                                info!("loaded rhai plugin: {}", name);
                                scripts.push((name, ast));
                            }
                            Err(err) => {
                                error!("failed to compile plugin {}: {}", path.display(), err);
                            }
                        }
                    }
                }
            }
        }

        Self { engine, scripts }
    }

    pub fn on_player_login(&self, player_name: &str, ip: &str) -> bool {
        for (name, ast) in &self.scripts {
            let mut scope = Scope::new();
            let res: Result<bool, _> = self.engine.call_fn(
                &mut scope,
                ast,
                "on_player_login",
                (player_name.to_string(), ip.to_string()),
            );
            if let Ok(allow) = res {
                if !allow {
                    info!("plugin '{}' denied login for player '{}'", name, player_name);
                    return false;
                }
            }
        }
        true
    }

    pub fn on_command(&self, player_name: &str, command: &str) -> Option<String> {
        for (_name, ast) in &self.scripts {
            let mut scope = Scope::new();
            let res: Result<String, _> = self.engine.call_fn(
                &mut scope,
                ast,
                "on_command",
                (player_name.to_string(), command.to_string()),
            );
            if let Ok(output) = res {
                if !output.is_empty() {
                    return Some(output);
                }
            }
        }
        None
    }
}
