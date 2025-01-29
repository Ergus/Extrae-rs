use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct GlobalConfig {
    pub(crate) counters: Vec<String>, // Example array
}

impl GlobalConfig {
    pub(crate) fn new() -> GlobalConfig
    {
        config::Config::builder()
            .set_default("counters", Vec::<String>::new())
            .expect("Failed to set default counters")
            .add_source(config::File::with_name("extrae").required(false))
            .add_source(config::Environment::with_prefix("EXTRAE")
                .ignore_empty(true)
                .try_parsing(true)
                .with_list_parse_key("counters")
                .ignore_empty(true)
                .list_separator(","))
            .build().unwrap()
            .try_deserialize::<GlobalConfig>()
            .unwrap()
    }
}


#[cfg(test)]
mod global_config {

    use super::*;
    use std::io::Write;

    struct TempFile {
        path: std::path::PathBuf,
        pub(crate) file: std::fs::File,
    }

    impl TempFile {
        fn new(name: &str) -> Self {

            let path = std::path::PathBuf::from(name);

            let file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true) // Truncate the file if it already exists
                .open(name)
                .expect("Failed creating file");

            Self { path, file }
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {

            if let Err(e) = std::fs::remove_file(&self.path) {
                eprintln!("Failed to delete temp file: {}", e);
            }
        }
    }

    #[test]
    fn global_config_constructors_default()
    {
        // This is a single test because tests are run as multithread.

        std::env::remove_var("EXTRAE_counters");

        // Test default constructor
        let config_default = GlobalConfig::new();
        assert_eq!(config_default.counters, Vec::<String>::new());

        // From environment
        std::env::set_var("EXTRAE_counters","111,222");
        let config_env = GlobalConfig::new();
        assert_eq!(config_env.counters, vec!["111", "222"]);
        std::env::remove_var("EXTRAE_counters");

        // From a file
        let mut temp_file = TempFile::new("extrae.toml");
        writeln!(temp_file.file, "counters = [\"333\", \"444\"]").expect("Failed to write");
        temp_file.file.flush().unwrap();

        let config_file = GlobalConfig::new();
        assert_eq!(config_file.counters, vec!["333", "444"]);

        // From file and environment
        std::env::set_var("EXTRAE_counters","111,222");
        let config_file2 = GlobalConfig::new();
        assert_eq!(config_file2.counters, vec!["111", "222"]);
        std::env::remove_var("EXTRAE_counters");
    }
}

