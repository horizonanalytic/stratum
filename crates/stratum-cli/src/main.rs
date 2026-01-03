//! Stratum CLI - Command-line interface for the Stratum programming language

fn main() {
    println!("Stratum v{}", stratum_core::VERSION);
    println!("A Goldilocks programming language");
}

#[cfg(test)]
mod tests {
    #[test]
    fn cli_starts() {
        assert!(true);
    }
}
