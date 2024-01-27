pub mod rendering;
use core::panic;

use std::collections::HashMap;
use std::fmt::Display;
use std::iter::once;

use ndarray::{Array, Array1, Array2, ArrayView1, Axis};
use rand::{rngs::ThreadRng, Rng};

/// Possible colors on the cube.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Color {
    White,
    Red,
    Blue,
    Yellow,
    Orange,
    Green,
}

// Implement `Display` for `Color`, so that we can print the colors to the console.
impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Color::White => "\x1b[30;47mW\x1b[0m",
            Color::Red => "\x1b[30;41mR\x1b[0m",
            Color::Blue => "\x1b[30;44mB\x1b[0m",
            Color::Yellow => "\x1b[30;103mY\x1b[0m",
            Color::Orange => "\x1b[30;43mO\x1b[0m",
            Color::Green => "\x1b[30;42mG\x1b[0m",
        };
        write!(f, "{}", s)
    }
}

/// Possible turn directions.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TurnDir {
    Clockwise,
    CounterClockwise,
}
impl TurnDir {
    /// Returns the reversal of the direction.
    ///
    /// If `self` is `Clockwise then returns `CounterClockwise` and vice versa.
    fn get_reversed(&self) -> TurnDir {
        match self {
            TurnDir::Clockwise => TurnDir::CounterClockwise,
            TurnDir::CounterClockwise => TurnDir::Clockwise,
        }
    }
}

/// Struct that holds the 2d matrix of colors.
#[derive(Debug, Clone)]
struct Face {
    colors: Array2<Color>,
    size: usize,
}
impl Face {
    /// Constructor that takes in the size and the initial color of the face.
    ///
    /// The face's matrix will always be a square. Panics (crashes) if size is 0.
    fn new(size: usize, color: Color) -> Face {
        if size == 0 {
            panic!("Size can't be 0");
        }
        Face {
            colors: Array::from_elem((size, size), color), // create array filled with `color`
            size,
        }
    }

    /// Rotates the color matrix 90-degree clockwise or counter-clockwise, depending on `turn_dir`.
    fn rotate(&mut self, turn_dir: TurnDir) {
        // transpose array
        self.colors.swap_axes(0, 1);
        // reverse column/row based on `turn_dir`
        match turn_dir {
            // invert the 0th axis which is the Y axis, meaning reversing each column
            TurnDir::Clockwise => self.colors.invert_axis(Axis(1)),
            // invert the 1st axis which is the X axis, meaning reversing each row
            TurnDir::CounterClockwise => self.colors.invert_axis(Axis(0)),
        }
    }

    /// Returns the ith row or column (depending on `is_column`) of the color matrix.
    ///
    /// `i` starts from `0`, and is ordered left to right and top to bottom.
    fn get_slice(&self, i: usize, is_column: bool) -> ArrayView1<Color> {
        if !is_column {
            return self.colors.row(i);
        }
        self.colors.column(i)
    }

    /// Sets the ith row or column (depending on `is_column`) of the color matrix.
    ///
    /// `i` starts from `0`, and is ordered left to right and top to bottom.
    /// Assumes that `slice` has the appropriate size and ordered left to right or top to bottom.
    fn set_slice(&mut self, i: usize, is_column: bool, slice: &ArrayView1<Color>) {
        if !is_column {
            self.colors.row_mut(i).assign(slice);
            return;
        }
        self.colors.column_mut(i).assign(slice);
    }

    /// Returns `true` if the color matrix all have the same color, `false` otherwise.
    ///
    /// Useful to check if the cube is solved.
    fn is_single_color(&self) -> bool {
        let reference_color = self.colors.iter().next().unwrap(); // get first color
        self.colors.iter().all(|c| c == reference_color)
    }
}
/// Implements `Display` for `Face`, allowing printing the color matrix to the console.
impl Display for Face {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        for y in 0..self.size {
            for x in 0..self.size {
                s.push_str(&format!("{}", self.colors[[y, x]]));
            }
            if y != self.size - 1 {
                s.push('\n');
            }
        }
        write!(f, "{s}")
    }
}

/// Possible axes of the cube.
///
/// `X` is going from left to right.
/// `Y` is going bottom to top.
/// `Z` is going from back to front.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CubeAxis {
    X,
    Y,
    Z,
}

/// Possible directions of the faces.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum FaceDir {
    Up,
    Down,
    Right,
    Left,
    Front,
    Back,
}
impl FaceDir {
    /// An array of all the face directions. Useful when we need to iterate through all of the
    /// directions.
    const ALL_FACE_DIR: [FaceDir; 6] = [
        FaceDir::Up,
        FaceDir::Down,
        FaceDir::Left,
        FaceDir::Right,
        FaceDir::Front,
        FaceDir::Back,
    ];

    /// Returns the array of directions surrounding (orthogonal to) the axis.
    /// The ordering of these directions in the array depends on `turn_dir`.
    ///
    /// The first direction is the one earliest alphabetically. The following directions rotates
    /// around the axis based on `turn_dir`. We're looking at the axis so that the positive
    /// direction is pointing towards us.
    fn get_dir_surrounding_axis(axis: CubeAxis, turn_dir: TurnDir) -> [FaceDir; 4] {
        let mut result = match axis {
            CubeAxis::X => [FaceDir::Back, FaceDir::Down, FaceDir::Front, FaceDir::Up],
            CubeAxis::Y => [FaceDir::Back, FaceDir::Right, FaceDir::Front, FaceDir::Left],
            CubeAxis::Z => [FaceDir::Down, FaceDir::Left, FaceDir::Up, FaceDir::Right],
        };
        if turn_dir == TurnDir::CounterClockwise {
            result.reverse()
        }
        result
    }

    /// Returns the axis of rotation and direction if we turn the face in `turn_dir`.
    ///
    /// For example, if `self` is `Up` and `turn_dir` is clockwise, we're turning the Y axis
    /// clockwise. so the function returns `(Axis::Y, TurnDir::Clockwise)`.
    /// Similarly, if we're turning `Down` clockwise, we're turning the Y axis counter-clockwise,
    /// thus returning `(Axis::Down, TurnDir::CounterClockwise).`
    fn get_rotate_axis_and_dir(self, turn_dir: TurnDir) -> (CubeAxis, TurnDir) {
        (
            self.get_axis(),
            if self.is_positive() {
                turn_dir
            } else {
                turn_dir.get_reversed()
            },
        )
    }

    /// Returns the axis this direction is in
    fn get_axis(&self) -> CubeAxis {
        match self {
            FaceDir::Up => CubeAxis::Y,
            FaceDir::Down => CubeAxis::Y,
            FaceDir::Right => CubeAxis::X,
            FaceDir::Left => CubeAxis::X,
            FaceDir::Front => CubeAxis::Z,
            FaceDir::Back => CubeAxis::Z,
        }
    }

    /// Returns whether this direction is pointing in the positive direction.
    fn is_positive(&self) -> bool {
        match self {
            FaceDir::Up => true,
            FaceDir::Down => false,
            FaceDir::Right => true,
            FaceDir::Left => false,
            FaceDir::Front => true,
            FaceDir::Back => false,
        }
    }

    /// Rotate the face direction in `turn_dir` along `axis`.
    fn apply_rotation(&mut self, axis: CubeAxis, turn_dir: TurnDir) {
        let surrounding_dirs = FaceDir::get_dir_surrounding_axis(axis, turn_dir);
        if let Some(i) = surrounding_dirs.iter().position(|fd| fd == self) {
            *self = surrounding_dirs[(i + 1) % 4];
        }
    }
}
impl Display for FaceDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            FaceDir::Up => "U",
            FaceDir::Down => "D",
            FaceDir::Right => "R",
            FaceDir::Left => "L",
            FaceDir::Front => "F",
            FaceDir::Back => "B",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone)]
pub struct Turn {
    face_dir: FaceDir,
    turn_dir: TurnDir,
}
impl Turn {
    pub fn new(face_dir: FaceDir, turn_dir: TurnDir) -> Turn {
        Turn { face_dir, turn_dir }
    }

    fn random_turn(rng: &mut ThreadRng) -> Turn {
        // make random turn
        let face_dir = FaceDir::ALL_FACE_DIR[rng.gen_range(0..6)];
        let turn_dir = if rng.gen_bool(0.5) {
            TurnDir::Clockwise
        } else {
            TurnDir::CounterClockwise
        };
        Turn { face_dir, turn_dir }
    }

    /// check if other is this turn but reversed
    pub fn is_reversed(&self, other: &Turn) -> bool {
        self.face_dir == other.face_dir && self.turn_dir == other.turn_dir.get_reversed()
    }

    pub fn algo_string(algo: &Vec<Turn>) -> String {
        algo.iter()
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
            .join(" ")
    }
}
impl Display for Turn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = self.face_dir.to_string();
        if self.turn_dir == TurnDir::CounterClockwise {
            s.push('\'');
        }
        write!(f, "{}", s)
    }
}

/// struct that models a cube
#[derive(Clone)]
pub struct Cube {
    /// array of `Face`, each element coresponding to a face on the cube.
    /// the direction of each element depends on `dir_order`, which depends on `Cube::INIT_CONFIG`
    faces: [Face; 6],
    /// direction order of `faces`, depends on `Cube::INIT_CONFIG`
    dir_order: [FaceDir; 6],
    /// Size the cube.
    size: usize,
}
impl Cube {
    /// Holds information about which face has which color initially.
    const INIT_CONFIG: [(FaceDir, Color); 6] = [
        (FaceDir::Up, Color::Yellow),
        (FaceDir::Down, Color::White),
        (FaceDir::Left, Color::Green),
        (FaceDir::Right, Color::Blue),
        (FaceDir::Front, Color::Orange),
        (FaceDir::Back, Color::Red),
    ];

    /// Create a new cube with `Cube::INIT_COFIG` configurations.
    pub fn new(size: usize) -> Cube {
        let mut faces = Vec::with_capacity(6);
        let mut dir_order = Vec::with_capacity(6);
        for (face_dir, color) in Cube::INIT_CONFIG {
            faces.push(Face::new(size, color));
            dir_order.push(face_dir);
        }
        Cube {
            faces: faces.try_into().unwrap(),
            dir_order: dir_order.try_into().unwrap(),
            size,
        }
    }

    fn get_dir_index(&self, face_dir: &FaceDir) -> usize {
        self.dir_order.iter().position(|fd| fd == face_dir).unwrap()
    }
    fn get_face(&self, face_dir: &FaceDir) -> &Face {
        &self.faces[self.get_dir_index(face_dir)]
    }

    fn get_face_mut(&mut self, face_dir: &FaceDir) -> &mut Face {
        &mut self.faces[self.get_dir_index(face_dir)]
    }

    /// Returns the order of the axes (where its pointing) of the face at `face_dir`
    ///
    /// For example, in the up face array, elements are indexed row first and then column. And
    /// since going top to bottom in front's perspective is the direction Front, the first axis is
    /// pointing Front. Similarly, the columns are going left to right, meaning pointing to the
    /// Right
    fn get_axes_order(face_dir: &FaceDir) -> [FaceDir; 2] {
        match face_dir {
            FaceDir::Up => [FaceDir::Front, FaceDir::Right],
            FaceDir::Down => [FaceDir::Back, FaceDir::Right],
            FaceDir::Right => [FaceDir::Down, FaceDir::Back],
            FaceDir::Left => [FaceDir::Down, FaceDir::Front],
            FaceDir::Front => [FaceDir::Down, FaceDir::Right],
            FaceDir::Back => [FaceDir::Down, FaceDir::Left],
        }
    }

    /// Rotate the band associated with `face_dir` in `turn_dir`.
    ///
    /// A band of a face is the colors that are rotated when we turn that face that aren't on the face
    /// itself.
    fn rotate_band(&mut self, turn: &Turn, layer: usize) {
        let (axis_of_rotation, rotate_dir) = turn.face_dir.get_rotate_axis_and_dir(turn.turn_dir);
        let is_positive = turn.face_dir.is_positive();

        // sometimes, the band includes the slice at the end of a face.
        // for example, when we turn the right face, the band would include the last column of
        // the front face.
        // this closure takes in a face direction `d` and returns a boolean which inidcates whether
        // to take the slice from the start or not.
        let check_is_from_start = |d| {
            for fd in Cube::get_axes_order(d).into_iter() {
                if fd.get_axis() == axis_of_rotation && fd.is_positive() != is_positive {
                    return true;
                }
            }
            false
        };

        // sometimes, the band includes the column of a face.
        // for example, when we turn the front face, the band would include the column of the
        // right and left face.
        // this closure takes in a face direction `d` and returns a boolean which indicates whether
        // to take its column or not.
        let check_is_column = |d| {
            let first_axis = Cube::get_axes_order(d)[0].get_axis();
            axis_of_rotation != first_axis
        };

        // sometimes, we need to reverse the slice before setting to the face.
        // for example, when we turn the front face, we should reverse the column from the right
        // face before setting the row of the bottom face to that slice.
        // this closure takes in a face direction `d` and returns a boolean which indicates whether
        // the slice should be reversed before setting it the face at `d`.
        // can't find an elegant solution that depends on `get_axes_order` yet
        let check_is_reversed = |&d| match axis_of_rotation {
            CubeAxis::X => {
                d == FaceDir::Back
                    || match rotate_dir {
                        TurnDir::Clockwise => d == FaceDir::Down,
                        TurnDir::CounterClockwise => d == FaceDir::Up,
                    }
            }
            CubeAxis::Y => false,
            CubeAxis::Z => match rotate_dir {
                TurnDir::Clockwise => d == FaceDir::Down || d == FaceDir::Up,
                TurnDir::CounterClockwise => d == FaceDir::Right || d == FaceDir::Left,
            },
        };

        // loop through all the face surrounding the axis that we are turning.
        let surrounding_dirs = FaceDir::get_dir_surrounding_axis(axis_of_rotation, rotate_dir);
        let mut prev_slice: Option<Array1<Color>> = None; // there's no previous slice before the
                                                          // loop starts.
        for d in surrounding_dirs.iter().chain(once(&surrounding_dirs[0]))
        // here we extends the iterator, adding the starting
        // direction to the end, since at the end we should
        // set the slice on the original face.
        {
            // for everty surrounding face, do the checks, and then get the appropriate slice.
            let i = if check_is_from_start(d) {
                layer - 1
            } else {
                self.size - layer
            };
            let is_column = check_is_column(d);
            let curr_slice = self.get_face(&d).get_slice(i, is_column).to_owned();

            // we set the appropriate slice the `prev_slice` if it exists.
            if let Some(mut ps) = prev_slice {
                if check_is_reversed(d) {
                    ps.invert_axis(Axis(0));
                }
                self.get_face_mut(&d).set_slice(i, is_column, &ps.view());
            }

            // updates `prev_slice`
            prev_slice = Some(curr_slice);
        }
    }

    /// Turn the face corresponding to `face_dir` on the cube in the direction indicated by `turn_dir`.
    pub fn turn_layer(&mut self, turn: &Turn, layer: usize) {
        if layer == 0 {
            panic!("layer must be nonzero. index starts at one (rubiks cube notation convention).");
        }
        if layer == 1 {
            self.get_face_mut(&turn.face_dir).rotate(turn.turn_dir);
        }
        self.rotate_band(turn, layer);
    }

    /// Returns true if all the faces on the cube each consist of only one color.
    pub fn is_solved(&self) -> bool {
        self.faces.iter().all(|face| face.is_single_color())
    }

    /// Scramble the cube with `k` random 90-degree turns. Returns the list of turns used to scramble.
    ///
    /// It's guaranteed that the turns would not cancel the immediately previous turn.
    pub fn scramble(&mut self, k: usize) -> Vec<Turn> {
        if self.size > 2 {
            panic!("scrambling not implemented for cubes larger than 2x2")
        }
        let mut algo = Vec::with_capacity(k);
        let mut prev_turn: Option<Turn> = None;
        let mut rng = rand::thread_rng();
        for _ in 0..k {
            let turn = loop {
                let turn_proposal = Turn::random_turn(&mut rng);
                if let Some(pt) = &prev_turn {
                    if turn_proposal.is_reversed(pt) {
                        continue;
                    }
                }
                break turn_proposal;
            };
            algo.push(turn.clone());
            // update `prev_turn` and apply random turn
            self.turn_layer(&turn, 1);
            prev_turn = Some(turn);
        }
        algo
    }

    /// Applies the list of turns to the cube.
    pub fn apply_algorithm(&mut self, algo: Vec<Turn>) {
        for turn in algo.iter() {
            self.turn_layer(turn, 1);
        }
    }

    /// Effectively changing which `Face` corresponds to which `FaceDir`
    pub fn rotate_whole_cube(&mut self, axis: CubeAxis, turn_dir: TurnDir) {
        for face_dir in self.dir_order.iter_mut() {
            face_dir.apply_rotation(axis, turn_dir);
        }
    }

    pub fn hamming_distance(&self, other: &Cube) -> usize {
        if self.size != other.size {
            panic!("Can't get hamming distance from 2 different sized cubes!");
        }
        let mut distance = 0;
        for i in 0..6 {
            let self_colors = self.faces[i].colors.iter();
            let other_colors = other.faces[i].colors.iter();
            // count the differences between two faces
            distance += self_colors
                .zip(other_colors)
                .filter(|&(sc, oc)| sc != oc)
                .count();
        }
        distance
    }

    pub fn all_possible_solved_cubes(size: usize) -> Vec<Cube> {
        let mut res = Vec::with_capacity(24); // there are 6*4=24 possible orientation of the cube
        let mut cube = Cube::new(size);
        for _ in 0..4 {
            for _ in 0..4 {
                res.push(cube.clone());
                cube.rotate_whole_cube(CubeAxis::Z, TurnDir::Clockwise);
            }
            cube.rotate_whole_cube(CubeAxis::Y, TurnDir::Clockwise);
        }
        cube.rotate_whole_cube(CubeAxis::X, TurnDir::Clockwise);
        for _ in 0..4 {
            res.push(cube.clone());
            cube.rotate_whole_cube(CubeAxis::Z, TurnDir::Clockwise);
        }
        cube.rotate_whole_cube(CubeAxis::X, TurnDir::Clockwise);
        cube.rotate_whole_cube(CubeAxis::X, TurnDir::Clockwise);
        for _ in 0..4 {
            res.push(cube.clone());
            cube.rotate_whole_cube(CubeAxis::Z, TurnDir::Clockwise);
        }
        res
    }
}

/// Implements `Display` for `Cube` so that we can print it in the console.
/// The prints the unfolded cube (its net). with the following format:
///   |U|
/// |L|F|R|B|
///   |D|
impl Display for Cube {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        let padding = 1;
        let mut face_repr = HashMap::new();
        for face_dir in FaceDir::ALL_FACE_DIR {
            face_repr.insert(face_dir, self.get_face(&face_dir).to_string());
        }
        let get_face_rows = |face_dir: FaceDir| face_repr.get(&face_dir).unwrap().split("\n");

        for row in get_face_rows(FaceDir::Up) {
            s.push_str(&" ".repeat(self.size + padding));
            s.push_str(row);
            s.push('\n');
        }
        s.push_str(&"\n".repeat(padding));

        for i in 0..self.size {
            for face_dir in [FaceDir::Left, FaceDir::Front, FaceDir::Right, FaceDir::Back] {
                s.push_str(get_face_rows(face_dir).nth(i).unwrap());
                s.push_str(&" ".repeat(padding));
            }
            s.push('\n');
        }
        s.push_str(&"\n".repeat(padding));

        for row in get_face_rows(FaceDir::Down) {
            s.push_str(&" ".repeat(self.size + padding));
            s.push_str(row);
            s.push('\n');
        }
        write!(f, "{s}")
    }
}
