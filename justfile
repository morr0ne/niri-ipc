# Default recipe
_:
    @just --list

# Build the project
build:
    cargo build --release

# Install binary and systemd service
install DESTDIR="" PREFIX="/usr":
    install -Dm755 target/release/piri {{DESTDIR}}{{PREFIX}}/bin/piri
    install -Dm644 resources/piri.service {{DESTDIR}}{{PREFIX}}/lib/systemd/user/piri.service

# Clean build artifacts
clean:
    cargo clean
