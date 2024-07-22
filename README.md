# TODO
- Maxlag/rate limits?

# Notes
## Code coverage
```bash
cargo install cargo-tarpaulin # Once
cargo tarpaulin -o html
```

## Lizard
Lizard is a simple code analyzer, giving cyclomatic complexity etc.
https://github.com/terryyin/lizard
```bash
lizard src -C 7 -V -L 35
```
