run package:
    cargo run --package {{package}}

fmt package="all":
    @echo "Formatting {{package}}"
    @if [ "{{package}}" = "all" ]; then \
        cargo fmt --all; \
    else \
        cargo fmt --package {{package}}; \
    fi
