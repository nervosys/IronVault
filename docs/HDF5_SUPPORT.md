# HDF5 Support

`.h5` and `.hdf5` files are supported in every build. No feature flag, no system
library, no extra setup.

```bash
iv store model.h5 --name my-model
iv get my-model --output restored.h5
```

## Why there is no `hdf5-support` feature

There used to be one. It gated an optional `hdf5` crate dependency that no code
in this repository ever called, so enabling it linked the HDF5 C library and
changed nothing observable. This page previously claimed that `.h5` files were
unsupported without it, which was never true. Both the flag and the unused
dependency were removed.

## What the vault does with an HDF5 file

The same thing it does with any model file: stores it as opaque encrypted bytes,
checksums it, and versions it. `ModelFormat::HDF5` is detected from the `.h5` /
`.hdf5` extension and drives compression-ratio estimation and conversion-path
lookup. Round-tripping is byte-exact.

## What is not supported

Tensor-level introspection. `iv diff` reports HDF5 differences at the file
level (size, checksum) rather than per-tensor, because nothing parses the HDF5
container's group/dataset structure. SafeTensors and GGUF do get tensor-level
diffs, since their headers can be read without a C library.

If per-tensor HDF5 diffing is wanted, that is the work to do — and it would be a
real feature flag, because it would genuinely need `libhdf5` (or a pure-Rust
reader) at build time.

## Related

- [FEATURE_FLAGS.md](FEATURE_FLAGS.md) — the flags that do exist
- [`FORMATS.md`](https://github.com/nervosys/IronVault/blob/master/FORMATS.md) — every supported format
