mod app;
mod cube;
mod search;
mod stats;

use app::main_app_loop;
use stats::{heuristic_stats::check_heuristic, idastar_stats::check_idastar};

/// main function, called when we starts.
fn main() {
    // check for feature stats flag
    if !cfg!(feature = "stats") {
        // run the rubiks cube app
        main_app_loop();
    } else {
        // run experiments
        check_idastar(&search::single_l0, "Single L0");
        check_idastar(&search::all_l0, "All L0");
        check_heuristic(&search::single_l0, "Single L0");
        check_heuristic(&search::all_l0, "All L0");
    }
}
