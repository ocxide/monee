pub mod host_dir {
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct HostDir(String);

    impl std::fmt::Display for HostDir {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl AsRef<str> for HostDir {
        fn as_ref(&self) -> &str {
            &self.0
        }
    }
}
