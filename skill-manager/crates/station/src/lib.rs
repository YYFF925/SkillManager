//! 统一管理站：登录、设备对接与转发。不得依赖 skill-manager-core。

pub mod auth;
pub mod db;
pub mod devices;
pub mod http;
pub mod proxy;
pub mod serve;

pub use serve::{default_data_dir, default_listen, run_blocking};

#[cfg(test)]
mod isolation {
    #[test]
    fn cargo_toml_does_not_depend_on_core() {
        let manifest = include_str!("../Cargo.toml");
        let deps = manifest
            .split("\n[")
            .find(|section| section.starts_with("dependencies]"))
            .unwrap_or("");
        assert!(
            !deps.contains("skill-manager-core"),
            "station must never depend on skill-manager-core"
        );
    }
}
