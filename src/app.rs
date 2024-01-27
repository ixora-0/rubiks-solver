use std::io::{stdin, stdout, Write};

use crate::{
    cube::rendering::CubeRender,
    cube::{Cube, FaceDir, Turn, TurnDir},
    search::{self, idastar},
};

/// Parses a string to a `FaceDir`.
///
/// Trims, ignore case of the input. Returns `None` if the input is not in "UDLRFB".j
fn string_to_face_dir(s: &String) -> Option<FaceDir> {
    match s.trim().to_uppercase().as_str() {
        "U" => Some(FaceDir::Up),
        "D" => Some(FaceDir::Down),
        "L" => Some(FaceDir::Left),
        "R" => Some(FaceDir::Right),
        "F" => Some(FaceDir::Front),
        "B" => Some(FaceDir::Back),
        _ => None,
    }
}

/// Takes a list of strings, parses and returns a list of `FaceDir` and `TurnDir` that the list of
/// strings represents
///
/// The format is "face_dir[' or 2]". `face_dir` indicates the face (U, D, L, R, F, or B). "'"
/// indicates a counter-clockwise turn and "2" indicates a 180-degree turn. Returns None if the
/// turns are invalid.
fn parse_algorithm(turns: Vec<&str>) -> Option<Vec<Turn>> {
    // create a macro to make parsing and return `None` if the element is invalid quicker.
    macro_rules! parse_or_return {
        ($e:expr) => {
            match string_to_face_dir($e) {
                Some(face_dir) => face_dir,
                None => return None,
            }
        };
    }

    // loop through the list of strings, and add to result the parsed move.
    let mut result = Vec::with_capacity(turns.len());
    for turn in turns.into_iter() {
        if turn.is_empty() {
            return None;
        }
        let trimmed_turn = &turn[0..turn.len() - 1].to_string();

        // check for possible post-fixes.
        match turn.to_uppercase().chars().last().unwrap() {
            '\'' => {
                result.push(Turn::new(
                    parse_or_return!(trimmed_turn),
                    TurnDir::CounterClockwise,
                ));
            }
            '2' => {
                result.push(Turn::new(
                    parse_or_return!(trimmed_turn),
                    TurnDir::Clockwise,
                ));
                result.push(Turn::new(
                    parse_or_return!(trimmed_turn),
                    TurnDir::Clockwise,
                ));
            }
            _ => result.push(Turn::new(
                parse_or_return!(&turn.to_string()),
                TurnDir::Clockwise,
            )),
        }
    }
    Some(result)
}

/// Runs the main app loop until the user input "Q"
pub fn main_app_loop() {
    // creates new cube.
    let mut cube = Cube::new(2);

    let (x_scale, y_scale, img_w, img_h) = (10.0, 5.0, 80, 29);
    let rotate_speed = 10_f32.to_radians();
    let mut cube_render = CubeRender::new(&cube, x_scale, y_scale, img_w, img_h);

    // loop forever until the user types "q".
    loop {
        // prints the cube.
        cube_render.render_cube();

        // prints prompt.
        println!(
            "Q to quit. X to reset. C to check if the cube is solved. M to scramble the cube."
        );
        println!("U/D/R/L/F/B to turn the corresponding face clockwise. Add ' to turn counter-clockwise.");
        println!("V + W/A/S/D to rotate the view");
        println!("S to find the solution for the cube using IDA*");
        print!("TYPE COMMAND: ");
        stdout().flush().expect("Error when printing text");

        // read line
        let mut cmd = String::new();
        stdin()
            .read_line(&mut cmd)
            .expect("Error when reading command");

        // match command
        match cmd.trim().to_uppercase().as_str() {
            "Q" => break, // if the command is "Q", break the main loop thus exiting.
            "X" => {
                // if the command is "X", set `cube` to a new cube, then update render
                cube = Cube::new(2);
                cube_render.update_colors(&cube);
            }
            //
            "C" => {
                // if command is "c", checks if cube is solved and prints the result
                // accordingly.
                if cube.is_solved() {
                    println!("The cube is solved");
                } else {
                    println!("The cube is not solved");
                }
            }

            "M" => {
                // if the command is "M", prompts a number then scramble.
                print!("Type number of turns to scramble: ");
                stdout().flush().expect("Error when printing text");
                let mut k = String::new();
                stdin()
                    .read_line(&mut k)
                    .expect("Error when reading command");
                let k: usize = k.trim().parse().expect("Can't parse to a number :(");
                let algo = cube.scramble(k);
                print!("Scramble sequence: ");
                println!("{}", Turn::algo_string(&algo));
                cube_render.update_colors(&cube);
            }

            "VW" => cube_render.rotate_pitch(rotate_speed),
            "VS" => cube_render.rotate_pitch(-rotate_speed),
            "VA" => cube_render.rotate_yaw(rotate_speed),
            "VD" => cube_render.rotate_yaw(-rotate_speed),

            "S" => {
                // if the command is "S", run IDA*
                let result = idastar(cube.clone(), &search::single_l0, true);
                println!("{result}");
            }

            // if it's none of the above:
            // check if we can parse the input into a list of moves. if we can't parse
            // (`parse_algorithm` returns `None`), prompt the user and execute the innter loop
            // again, thus prompting the user's input again. else if we can parse it then apply
            // the turns to the cube.
            _ => match parse_algorithm(cmd.split_whitespace().collect()) {
                None => {
                    println!("Invalid command");
                    continue;
                }
                Some(algo) => {
                    cube.apply_algorithm(algo);
                    cube_render.update_colors(&cube);
                }
            },
        }
    }
}
