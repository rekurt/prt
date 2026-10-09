#!/usr/bin/env bash
set -euo pipefail
version=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "prt") | .version')
binary=target/x86_64-unknown-linux-gnu/release/prt
mkdir -p dist package-root/usr/bin package-root/usr/share/doc/prt
install -m 755 "$binary" package-root/usr/bin/prt
install -m 644 LICENSE README.md package-root/usr/share/doc/prt/
ldd "$binary"
common=(-s dir -n prt -v "$version" -C package-root --license MIT
  --url https://github.com/rekurt/prt --maintainer 'rekurt <security@rekurt.dev>'
  --description 'Live network port inspector with process details, SSH tunnel health and JSON/CSV export')
fpm "${common[@]}" -t deb -a amd64 --depends 'libc6 >= 2.35' --depends libgcc-s1 \
  -p "dist/prt_${version}_amd64.deb" usr
fpm "${common[@]}" -t rpm -a x86_64 --depends 'glibc >= 2.35' --depends libgcc \
  -p "dist/prt-${version}-1.x86_64.rpm" usr
# Install the actual packages with the distribution package manager.
for image in ubuntu:22.04 debian:12; do
  docker run --rm -e PRT_VERSION="$version" -v "$PWD/dist:/packages:ro" "$image" bash -euc '
    apt-get update -qq
    apt-get install -y /packages/*.deb python3
    test "$(prt --version)" = "prt $PRT_VERSION"
    prt --help >/dev/null
    prt --export json | python3 -c "import json,sys; assert isinstance(json.load(sys.stdin), list)"
  '
done
docker run --rm -e PRT_VERSION="$version" -v "$PWD/dist:/packages:ro" fedora:43 bash -euc '
  dnf install -y /packages/*.rpm python3
  test "$(prt --version)" = "prt $PRT_VERSION"
  prt --help >/dev/null
  prt --export json | python3 -c "import json,sys; assert isinstance(json.load(sys.stdin), list)"
'
