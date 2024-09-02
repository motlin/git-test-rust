default:
    cargo fmt
    # cargo build --release
    cargo build
    cp ./target/debug/git-test ~/bin
