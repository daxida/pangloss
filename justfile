run:
  cargo run --release -- tmp/out1.json tmp/final.json --rformat=json --wformat=json

build:
  cargo build --release

clippy *args:
  cargo clippy {{args}} --all-targets --all-features -- -W clippy::nursery -W clippy::pedantic \
  -A clippy::must_use_candidate \
  -A clippy::module_name_repetitions \
  -A clippy::cast_precision_loss \
  -A clippy::unicode_not_nfc \
  -A clippy::wildcard_imports \
  -A clippy::missing_errors_doc \
  -A clippy::cast_possible_truncation

alias r := run

py:
  NO_GIT_VERSION=1 uv pip install "git+https://github.com/daxida/pyglossary.git@yomitan-deinf"
