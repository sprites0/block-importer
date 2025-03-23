use clap::Parser;
use lz4_flex::frame::FrameDecoder;
use rmp;
use rocksdb::DB;
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use types::{BlockAndReceipts, EvmBlock};
mod types;

#[derive(Parser)]
struct Args {
    /// Path to the raw HyperEVM data file
    #[clap(short, long)]
    ingest_dir: PathBuf,

    /// Path to the Hyperliquid DB base directory
    #[clap(short, long, default_value=default_db_dir().into_os_string())]
    db_dir: PathBuf,

    /// Start block number
    #[clap(short, long)]
    start_block: u64,

    /// End block number
    #[clap(short, long)]
    end_block: u64,
}

fn decompress(data: &[u8]) -> Result<Vec<u8>, lz4_flex::frame::Error> {
    let mut decoder = FrameDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

fn collect_block(ingest_dir: &Path, height: u64) -> (Vec<u8>, Vec<types::BlockAndReceipts>) {
    let f = ((height - 1) / 1_000_000) * 1_000_000;
    let s = ((height - 1) / 1_000) * 1_000;
    let path = format!("{}/{f}/{s}/{height}.rmp.lz4", ingest_dir.to_string_lossy());
    if Path::new(&path).exists() {
        let content = decompress(&std::fs::read(path).unwrap()).unwrap();
        let blocks: Vec<BlockAndReceipts> = rmp_serde::from_read(content.as_slice()).unwrap();
        (content, blocks)
    } else {
        panic!("Block not found: {height}, path: {path}");
    }
}

const CHUNK_SIZE: usize = 10000;

fn main() {
    let args = Args::parse();
    let db_dir = args.db_dir.join("/hyperliquid_data/db_hub/Rpc");
    let ingest_dir = args.ingest_dir;
    let db = DB::open_default(db_dir).unwrap();

    let mut block_key = [b'E', b'b', 0, 0, 0, 0, 0, 0, 0, 0];
    let mut blockhash_key = [
        b'E', b'n', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0,
    ];
    let mut txhash_key = [
        b'E', b't', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0,
    ];
    let mut buf = Vec::with_capacity(100);

    for i in (args.start_block..=args.end_block).step_by(CHUNK_SIZE) {
        let start = i;
        let end = std::cmp::min(i + (CHUNK_SIZE as u64), args.end_block);
        println!("Processing blocks {} to {}", start, end);

        for block_number in start..end {
            let (block_content, blocks) = collect_block(&ingest_dir, block_number);
            assert!(blocks.len() == 1);

            let EvmBlock::Reth115(block) = &blocks[0].block;
            let block_hash = block.hash();

            // [1..] because the first one is the array marker with length 1
            block_key[2..].copy_from_slice(block_number.to_be_bytes().as_ref());
            db.put(&block_key, &block_content[1..]).unwrap();

            blockhash_key[2..].copy_from_slice(block_hash.as_slice());
            buf.clear();
            rmp::encode::write_uint(&mut buf, block_number).unwrap();
            db.put(&blockhash_key, &buf).unwrap();

            for (tx_index, tx) in block.body().transactions().enumerate() {
                txhash_key[2..].copy_from_slice(tx.hash().as_slice());
                buf.clear();
                rmp::encode::write_array_len(&mut buf, 2).unwrap();
                rmp::encode::write_uint(&mut buf, block_number).unwrap();
                rmp::encode::write_uint(&mut buf, tx_index as u64).unwrap();

                db.put(&txhash_key, &buf).unwrap();
            }
        }
    }
    println!("Done");
}

fn default_db_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .expect("no home directory found")
        .join("hl")
}
