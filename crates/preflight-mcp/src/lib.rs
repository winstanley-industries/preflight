pub fn hello() -> &'static str {
    preflight_core::hello()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {
        assert_eq!(hello(), "hello preflight");
    }
}
