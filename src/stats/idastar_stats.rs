use std::{
    fs::{File, OpenOptions},
    io::{stdout, Write},
    ops::Range,
};

use csv::Writer;

use crate::{
    cube::{Cube, Turn},
    search::{idastar, SearchResult},
};

const NUM_PER_SCRAMBLE: usize = 16;
const NUM_MOVE_PER_SCRAMBLE_RANGE: Range<usize> = 1..10;
const CSV_FILE_PATH: &str = "idastar_stats.csv";

struct Data {
    scramble: Vec<Turn>,
    scramble_len: usize,
    search_result: SearchResult,
}

#[allow(dead_code)]
pub fn check_idastar(heuristic_function: &dyn Fn(&Cube) -> f32, heuristic_function_name: &str) {
    let mut data = Vec::new();
    for m in NUM_MOVE_PER_SCRAMBLE_RANGE {
        for i in 0..NUM_PER_SCRAMBLE {
            print!("\rScrambling with {m} moves, {i}/{NUM_PER_SCRAMBLE}     ");
            stdout().flush().expect("Error printing progress");
            let mut cube = Cube::new(2);
            let scramble = cube.scramble(m);
            let search_result = idastar(cube, heuristic_function, false);
            data.push(Data {
                scramble,
                scramble_len: m,
                search_result,
            });
        }
    }
    println!("\nWriting data to file");

    // determine if the file exists
    let file_exists = std::path::Path::new(CSV_FILE_PATH).exists();

    // open the file in write or append mode depending on whether it exists
    let file = if file_exists {
        OpenOptions::new()
            .write(true)
            .append(true)
            .open(CSV_FILE_PATH)
            .expect("Can't open file")
    } else {
        File::create(CSV_FILE_PATH).expect("Can't create file")
    };

    // write the header row if the file is newly created
    let mut csv_writer = Writer::from_writer(file);
    if !file_exists {
        csv_writer
            .write_record(&[
                "Scramble",
                "Scramble Length",
                "Solution",
                "Solution Length",
                "Heuristic Type",
                "Wall Time (ns)",
                "Node Visited",
            ])
            .expect("Error when writing header");
    }

    // write data to csv
    for d in data {
        csv_writer
            .write_record([
                Turn::algo_string(&d.scramble),
                d.scramble_len.to_string(),
                match &d.search_result.solution {
                    None => "".to_string(),
                    Some(algo) => Turn::algo_string(algo),
                },
                match &d.search_result.solution_len {
                    None => "".to_string(),
                    Some(l) => l.to_string(),
                },
                heuristic_function_name.to_string(),
                d.search_result.wall_time.as_nanos().to_string(),
                d.search_result.node_visited.to_string(),
            ])
            .expect("Error when trying to write row");
    }
    csv_writer.flush().expect("Error when flushing writer");
    println!("Done!");
}
