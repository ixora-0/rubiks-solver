use std::{
    cell::RefCell,
    fmt::Display,
    io::{stdout, Write},
    rc::Rc,
    time::{Duration, Instant},
};

use crate::cube::{Cube, FaceDir, Turn, TurnDir};

#[derive(Clone)]
struct Node {
    state: Cube,
    prev_action: Option<Turn>,
    parent: Option<Rc<RefCell<Node>>>,
    path_cost: usize,
    evaluation: Option<usize>, // acts as a cache to avoid recalculating heuristic
}
impl Node {
    fn new_root(init_cube: Cube) -> Node {
        Node {
            state: init_cube,
            prev_action: None,
            parent: None,
            path_cost: 0,
            evaluation: None,
        }
    }

    fn is_goal(&self) -> bool {
        self.state.is_solved()
    }

    fn get_path(&self) -> Vec<Turn> {
        if let Some(parent_node) = &self.parent {
            let mut path = parent_node.borrow().get_path();
            if let Some(action) = &self.prev_action {
                path.push(action.clone());
            }
            return path;
        }
        Vec::new()
    }

    fn get_evaluation(&mut self, heuristic_function: &dyn Fn(&Cube) -> f32) -> usize {
        if let Some(f) = self.evaluation {
            return f;
        }
        let new_h = heuristic_function(&self.state).ceil() as usize;
        match &self.parent {
            None => new_h,
            Some(ptr) => {
                let mut parent = ptr.borrow_mut();
                let new_f = self.path_cost + new_h;

                let parent_f = parent.get_evaluation(heuristic_function);

                // ensure that f is monotone (Korf pg. 104)
                let f = usize::max(new_f, parent_f);
                self.evaluation = Some(f);
                f
            }
        }
    }

    /// Returns the children of this node. Also consumes `self`.
    fn generate_children(parent_ptr: Rc<RefCell<Node>>) -> Vec<Node> {
        let mut res = Vec::with_capacity(5);
        let parent = parent_ptr.borrow();
        // only iterate through the two directions of right, up, and front, since the other turns
        // can be applied by these 6 turns.
        for face_dir in [FaceDir::Right, FaceDir::Up, FaceDir::Front] {
            for turn_dir in [TurnDir::Clockwise, TurnDir::CounterClockwise] {
                let turn = Turn::new(face_dir, turn_dir);
                // skip the reverse turn of the previous action, effectively cutting the branching
                // factor down to 5
                if let Some(t) = &parent.prev_action {
                    if t.is_reversed(&turn) {
                        continue;
                    }
                }

                let mut new_cube = parent.state.clone();
                new_cube.turn_layer(&turn, 1);

                res.push(Node {
                    state: new_cube,
                    prev_action: Some(turn),
                    parent: Some(Rc::clone(&parent_ptr)),
                    path_cost: parent.path_cost + 1,
                    evaluation: None,
                });
            }
        }
        res
    }
}

#[allow(dead_code)]
pub fn single_l0(cube: &Cube) -> f32 {
    cube.hamming_distance(&Cube::new(2)) as f32 / 12.0
}

#[allow(dead_code)]
pub fn all_l0(cube: &Cube) -> f32 {
    // have a cache to avoid creating the vec many time
    // cut heuristic evaluating time by half
    static mut ALL_POSSIBLE_SOLVED_CUBES_CACHE: Option<Vec<Cube>> = None;
    let apsc_iter = unsafe {
        if let Some(cache) = &ALL_POSSIBLE_SOLVED_CUBES_CACHE {
            cache.iter()
        } else {
            ALL_POSSIBLE_SOLVED_CUBES_CACHE = Some(Cube::all_possible_solved_cubes(2));
            ALL_POSSIBLE_SOLVED_CUBES_CACHE.as_ref().unwrap().iter()
        }
    };

    let mut min_dist = usize::MAX;
    for goal_state in apsc_iter {
        min_dist = usize::min(min_dist, cube.hamming_distance(&goal_state));
    }
    min_dist as f32 / 12.0
}

pub struct SearchResult {
    pub solution: Option<Vec<Turn>>,
    pub solution_len: Option<usize>,
    pub node_visited: usize,
    pub wall_time: Duration,
}
impl Display for SearchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Solution: {}\tWall Time: {} ns\tNode Visited: {}",
            match &self.solution {
                None => "Can't find solution".to_string(),
                Some(algo) => Turn::algo_string(algo),
            },
            self.wall_time.as_nanos(),
            self.node_visited
        )
    }
}

/// Based on Korf's
pub fn idastar(
    init_cube: Cube,
    heuristic_function: &dyn Fn(&Cube) -> f32,
    print_progress: bool,
) -> SearchResult {
    let mut root = Node::new_root(init_cube);
    const GIVE_UP_LIMIT: usize = 28;
    let mut limit = root.get_evaluation(heuristic_function);

    let mut node_visited = 0;
    let start_time = Instant::now();

    loop {
        let mut node_stack = vec![root.clone()];
        if print_progress {
            print!("\rSearching with limit = {limit:<10.2}");
        }
        stdout().flush().expect("Error when printing text");

        let mut min_f = usize::MAX;

        while !node_stack.is_empty() {
            let mut node = node_stack.pop().unwrap();
            node_visited += 1;
            let f = node.get_evaluation(heuristic_function);

            // println!(
            //     "{} {}",
            //     Turn::algo_string(&node.get_path()),
            //     node.get_evaluation(heuristic_function)
            // );
            // check for if node exceeds the threshold, if yes we skip it
            if f > limit {
                min_f = usize::min(min_f, f);
                continue;
            }

            // if we found the solution, returns the list of actions
            if node.is_goal() {
                if print_progress {
                    println!();
                }
                let path = node.get_path();
                return SearchResult {
                    solution_len: Some(path.len()),
                    solution: Some(path),
                    node_visited,
                    wall_time: start_time.elapsed(),
                };
            }

            // add children to node_stack
            let node_ptr = Rc::new(RefCell::new(node));
            node_stack.extend(Node::generate_children(node_ptr).into_iter());
        }
        // increase the limit
        limit = min_f;
        if limit > GIVE_UP_LIMIT {
            break;
        }
    }
    // can't find solution
    if print_progress {
        println!();
    }
    SearchResult {
        solution: None,
        solution_len: None,
        node_visited,
        wall_time: start_time.elapsed(),
    }
}
