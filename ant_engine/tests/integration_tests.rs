#[cfg(test)]
mod server_tests {
    use ant_engine::server::NetworkerHandle;

    #[test]
    fn test_networker() {
        let handle = NetworkerHandle::new("127.0.0.1:12345".to_string());
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(handle.receive_commands().is_empty());
    }
}
