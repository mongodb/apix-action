binary := "sync"
manifest := "sync/Cargo.toml"

build:
	cargo build --release --manifest-path {{manifest}}
	mkdir -p bin
	cp sync/target/release/{{binary}} bin/{{binary}}

build-debug:
	cargo build --manifest-path {{manifest}}
	mkdir -p bin
	cp sync/target/debug/{{binary}} bin/{{binary}}

sync *args: build
	./bin/{{binary}} {{args}}

sync-with-gh-token *args:
	GH_TOKEN="$(gh auth token)" just sync {{args}}
