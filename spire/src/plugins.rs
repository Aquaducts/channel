use crate::config::CONFIG;
use common::Job;
pub use common::{database::Database, PluginInfo};
use futures::Future;
use once_cell::sync::Lazy;
use std::{fs::read_to_string, pin::Pin, sync::Arc};

pub type JobCreateFunc =
    unsafe extern "C" fn(Arc<Database>, Job) -> Pin<Box<dyn Future<Output = ()>>>;

pub enum PluginType {
    JobCreate(JobCreateFunc),
    Unkown,
}

pub struct Plugin {
    pub meta: PluginInfo,
    pub events: Vec<PluginType>,
}

pub const PLUGINS: once_cell::sync::Lazy<Vec<Plugin>> = Lazy::new(|| {
    let mut plugins = Vec::new();
    // TODO: uhm fix this path
    let config = CONFIG.to_owned();

    if let Some(dir) = config.plugins_directory {
        let mut dir = std::fs::read_dir(dir).unwrap().into_iter();
        while let Some(Ok(entry)) = dir.next() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };

            if !file_type.is_dir() {
                continue;
            }

            let entry_path = entry.path().display().to_string();
            let plugin_path = format!("{}/meta.json", &entry_path);
            let meta =
                serde_json::from_str::<PluginInfo>(&read_to_string(plugin_path).unwrap()).unwrap();
            let mut events = Vec::new();
            unsafe {
                let plugin =
                    libloading::Library::new(format!("{}/{}", &entry_path, &meta.main_file))
                        .unwrap();

                for event in &meta.events {
                    // In the real thing this will be an enum
                    match event.as_str() {
                        "job_create" => {
                            let job_create_event: libloading::Symbol<JobCreateFunc> =
                                plugin.get(b"job_create").unwrap();

                            events.push(PluginType::JobCreate(*job_create_event));
                        }
                        _ => {}
                    }
                }
            }
            plugins.push(Plugin { events, meta });
        }
    }

    plugins
});
