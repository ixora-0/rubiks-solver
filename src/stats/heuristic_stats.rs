use std::{
    fs::{File, OpenOptions},
    io::{stdout, Write},
    ops::Range,
    time::{Duration, Instant},
};

use csv::Writer;

use crate::cube::{Cube, Turn};

const NUM_PER_SCRAMBLE: usize = 1000;
const NUM_MOVE_PER_SCRAMBLE_RANGE: Range<usize> = 1..30;
const CSV_FILE_PATH: &str = "heuristic_data.csv";

struct Data {
    scramble: Vec<Turn>,
    scramble_len: usize,
    heuristic: f32,
    wall_time: Duration,
}

#[allow(dead_code)]
pub fn check_heuristic(heuristic_function: &dyn Fn(&Cube) -> f32, heuristic_function_name: &str) {
    let mut data = Vec::new();
    for m in NUM_MOVE_PER_SCRAMBLE_RANGE {
        for i in 0..NUM_PER_SCRAMBLE {
            print!("\rScrambling with {m} moves, {i}/{NUM_PER_SCRAMBLE}     ");
            stdout().flush().expect("Error printing progress");
            let mut cube = Cube::new(2);
            let scramble = cube.scramble(m);
            let scramble_len = scramble.len();
            let start_time = Instant::now();
            let heuristic = heuristic_function(&cube);
            let wall_time = start_time.elapsed();
            data.push(Data {
                scramble,
                scramble_len,
                heuristic,
                wall_time,
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
                "Heuristic Type",
                "Heuristic",
                "Wall Time (ns)",
            ])
            .expect("Error when writing header");
    }

    // write data to csv
    for d in data {
        csv_writer
            .write_record([
                Turn::algo_string(&d.scramble),
                d.scramble_len.to_string(),
                heuristic_function_name.to_string(),
                d.heuristic.to_string(),
                d.wall_time.as_nanos().to_string(),
            ])
            .expect("Error when writing row");
    }
    csv_writer.flush().expect("Error when flushing writer");
    println!("Done!");
}
