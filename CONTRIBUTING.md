# Contributing

Thank you for helping improve `serde-shape`.

## Development workflow

Repository tasks are defined by `cargo x`:

```console
cargo x build --locked
cargo x test
cargo x lint
cargo x package --locked
```

Run `cargo x --help` or `cargo x <command> --help` for command-specific options. `cargo x lint --fix` applies supported formatting and lint fixes.

The main crate is `no_std` by default. Changes to core shape construction must continue to compile without the `std` feature; the test task exercises the supported feature combinations.

## Tests

Add tests at the observable behavior boundary:

* Put shape context, built-in type, and graph behavior tests in `serde-shape/src/tests.rs`.
* Put derive behavior and Serde attribute compatibility tests in `tests/derive`.
* Put end-to-end consumer scenarios and comparisons with actual Serde calls in `tests/integration`.
* Put regressions that specifically exercise `no_std` derive output in `tests/no_std`.

Prefer focused assertions for the contract under test. Avoid snapshots of entire debug graphs: they obscure the behavior being protected and make unrelated metadata additions expensive to review.

## Public API changes

Serialization and deserialization may have different shapes, bounds, and names. Check both directions when changing a shared implementation, and compare built-in types with Serde's actual data-model calls when behavior depends on the format or human-readable mode.

Update the README or crate-level documentation when a change affects setup, feature flags, supported representations, model boundaries, or migration steps.

Keep pull requests and commits focused enough to review independently. Explain the concrete user problem and avoid adding generic abstractions without a current consumer.
