default:
    cargo fmt
    cargo test
    cargo build
    # cargo build --release
    # cp ./target/debug/git-test ~/bin

cargo-fix:
    cargo fix --lib -p git_test --allow-dirty
    cargo fix --lib -p git_test --allow-dirty --tests
