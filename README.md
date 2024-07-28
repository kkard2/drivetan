# drivetan
Generates a meta directory structure to remember files on unplugged drives.

# Usage
```
Generates a meta directory structure to remember files on unplugged drives.
Standard output lists properly processed files separated by newlines

Usage: drivetan [OPTIONS] <SOURCE> <DESTINATION>

Arguments:
  <SOURCE>
  <DESTINATION>

Options:
  -m, --max-size <SIZE_IN_BYTES>  Max size in bytes to copy file unchanged [default: 0]
  -e, --extension <EXTENSION>     Extension for meta files [default: .drivetan.txt]
      --magic <MAGIC>             Magic at the start of a meta file [default: DRIVETAN]
      --skip-file <PATH>          Skip files/directories matching regexes in the provided file,
                                  separated by newlines (e.g. "\.git").
                                  Empty folders do not have trailing slashes
  -h, --help                      Print help
  -V, --version                   Print version
```

# Building
```
cargo build
```

# Installing
```
cargo install --path .
```
