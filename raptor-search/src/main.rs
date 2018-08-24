extern crate rocksdb;
extern crate fst;
extern crate raptor;
extern crate elapsed;

use std::env;
use std::str::from_utf8_unchecked;
use std::io::{self, Write};
use elapsed::measure_time;
use fst::Streamer;
use rocksdb::{DB, IngestExternalFileOptions};
use raptor::{Metadata, RankedStream, LevBuilder};

fn search(metadata: &Metadata, database: &DB, lev_builder: &LevBuilder, query: &str) {
    let mut automatons = Vec::new();
    for query in query.split_whitespace() {
        let lev = lev_builder.get_automaton(query);
        automatons.push(lev);
    }

    let map = metadata.as_map();
    let indexes = metadata.as_indexes();

    let mut stream = RankedStream::new(&map, &indexes, automatons, 20);
    while let Some(document) = stream.next() {
        print!("{:?}", document.document_id);

        let title_key = format!("{}-title", document.document_id);
        let title = database.get(title_key.as_bytes()).unwrap().unwrap();
        let title = unsafe { from_utf8_unchecked(&title) };
        print!(" {:?}", title);

        println!();
    }
}

fn main() {
    let name = env::args().nth(1).expect("Missing meta file name (e.g. lucid-ptolemy)");
    let map_file = format!("{}.map", name);
    let idx_file = format!("{}.idx", name);
    let sst_file = format!("{}.sst", name);

    let rocksdb = "rocksdb/storage";

    let (elapsed, meta) = measure_time(|| unsafe {
        Metadata::from_paths(map_file, idx_file).unwrap()
    });
    println!("{} to load metadata", elapsed);

    let (elapsed, db) = measure_time(|| {
        let db = DB::open_default(rocksdb).unwrap();
        db.ingest_external_file(&IngestExternalFileOptions::new(), &[&sst_file]).unwrap();
        db
    });
    println!("{} to load the RocksDB database", elapsed);

    let (elapsed, lev_builder) = measure_time(|| LevBuilder::new());
    println!("{} to load the levenshtein automaton", elapsed);

    loop {
        print!("Searching for: ");
        io::stdout().flush().unwrap();

        let mut query = String::new();
        io::stdin().read_line(&mut query).unwrap();
        let query = query.trim().to_lowercase();

        if query.is_empty() { break }

        let (elapsed, _) = measure_time(|| search(&meta, &db, &lev_builder, &query));
        println!("Finished in {}", elapsed);
    }
}