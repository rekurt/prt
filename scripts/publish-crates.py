#!/usr/bin/env python3
"""Publish workspace crates in order, checking the sparse index on retries."""
import json
import subprocess
import time
import urllib.error
import urllib.request


def indexed(crate, version):
    path = f"{crate[:2]}/{crate[2:4]}/{crate}" if len(crate) >= 4 else f"3/{crate[0]}/{crate}"
    request = urllib.request.Request(f"https://index.crates.io/{path}", headers={"Cache-Control": "no-cache"})
    # Retry transient transport/index failures, never treat them as an absent release.
    for attempt in range(5):
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                records = [json.loads(line) for line in response.read().decode().splitlines()]
            return any(record["vers"] == version and not record["yanked"] for record in records)
        except urllib.error.HTTPError as error:
            if error.code == 404:
                return False
            if error.code != 429 and error.code < 500:
                raise
            if attempt == 4:
                raise
        except urllib.error.URLError:
            if attempt == 4:
                raise
        time.sleep(5 * (attempt + 1))
    raise RuntimeError("Index check failed")


def main():
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"]))
    packages = {package["name"]: package for package in metadata["packages"]}
    version = packages["prt"]["version"]
    if packages["prt-core"]["version"] != version:
        raise RuntimeError("Workspace crate versions differ")
    for crate in ("prt-core", "prt"):
        if indexed(crate, version):
            print(f"{crate} {version} already published", flush=True)
        else:
            subprocess.run(["cargo", "publish", "--locked", "-p", crate], check=True)
        for attempt in range(60):
            if indexed(crate, version):
                break
            if attempt == 59:
                raise RuntimeError(f"{crate} {version} did not become available in the index")
            time.sleep(10)


if __name__ == "__main__":
    main()
