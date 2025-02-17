mod cfr;
mod game_kuhn;
mod interface;
mod util;

use cfr::CFRMinimizer;
use game_kuhn::KuhnGame;
use util::*;

//compute処理計測用
use std::time::Instant;

fn main() {
    let num_iterations = 10000;
    let kuhn_game = KuhnGame::new();
    let mut cfr = CFRMinimizer::new(&kuhn_game);

    let start = Instant::now(); // 計測開始
    let strategy = cfr.compute(num_iterations);
    let duration = start.elapsed(); // 計測終了

    let ev = compute_ev(&kuhn_game, 0, &strategy);
    let exploitability = compute_exploitability(&kuhn_game, &strategy);

    let num_private_hands = 3;
    let num_terminal_nodes = cfr.get_terminal_nodes();
    let complexity = compute_complexity(num_private_hands, num_terminal_nodes, exploitability);
    println!("計算量: {}",complexity);
    println!("処理時間: {:.3}秒", duration.as_secs_f64());

    println!();
    println!("[Kuhn Poker]");

    
    println!("- Exploitability: {:+.3e}", exploitability);

    println!();
    println!("[First player]");
    println!("- EV: {:+.4}", ev);

    for (history, history_str, action_str) in [
        (vec![], "", "Bet"),
        (vec![0, 1], "(Check => Bet => ?)", "Call"),
    ] {
        println!("- {}% {}", action_str, history_str);
        for i in 0..3 {
            println!(
                "    {}: {:.2}%",
                ["J", "Q", "K"][2 - i],
                100.0 * strategy[&history][1][2 - i]
            );
        }
    }

    println!();
    println!("[Second player]");
    println!("- EV: {:+.4}", -ev);

    for (history, history_str, action_str) in [
        (vec![0], "(Check => ?)", "Bet"),
        (vec![1], "(Bet => ?)", "Call"),
    ] {
        println!("- {}% {}", action_str, history_str);
        for i in 0..3 {
            println!(
                "    {}: {:.2}%",
                ["J", "Q", "K"][2 - i],
                100.0 * strategy[&history][1][2 - i]
            );
        }
    }
}
