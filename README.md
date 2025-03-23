# block-importer

Backfills [Raw HyperEVM block data](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/hyperevm/raw-hyperevm-block-data) into [hl-node](https://github.com/hyperliquid-dex/node) binary, so that Hyperliquid node can serve historical blocks.

```sh
$ cargo run --release -- --start-block 1 --end-block 30000 --ingest-dir ~/evm-blocks
# Optionally specify hyperliquid data directory
$ cargo run --release -- --start-block 1 --end-block 30000 --ingest-dir ~/evm-blocks --db-dir $HOME/hl
```
