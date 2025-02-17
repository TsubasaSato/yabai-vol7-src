mod cfr;
mod game_push_fold;
mod interface;
mod util;

use cfr::CFRMinimizer;
use game_push_fold::PushFoldGame;
use util::*;

//compute処理計測用
use std::time::Instant;

fn main() {
    let effective_stack = 10.0;
    let num_iterations = 1000;

    let push_fold_game = PushFoldGame::new(effective_stack);
    let mut cfr = CFRMinimizer::new(&push_fold_game);
    
    let start = Instant::now(); // 計測開始
    let strategy = cfr.compute(num_iterations);
    let duration = start.elapsed(); // 計測終了

    let ev = compute_ev(&push_fold_game, 0, &strategy);
    let exploitability = compute_exploitability(&push_fold_game, &strategy);

    let num_private_hands = 52 * 51 / 2;
    let num_terminal_nodes = cfr.get_terminal_nodes();
    let complexity = compute_complexity(num_private_hands, num_terminal_nodes, exploitability);
    println!("計算量: {}",complexity);
    println!("処理時間: {:.3}秒", duration.as_secs_f64());


    // Publicヒストリーを作成している？
    let pusher = &strategy[&vec![]];
    let caller = &strategy[&vec![1]];

    let mut push_rate = vec![vec![0.0; 13]; 13];
    let mut call_rate = vec![vec![0.0; 13]; 13];
    let mut overall_push_rate = 0.0;
    let mut overall_call_rate = 0.0;

    let mut k = 0;
    for i in 0..51 {
        for j in (i + 1)..52 {
            let rank1 = i / 4;
            let rank2 = j / 4;
            let suit1 = i % 4;
            let suit2 = j % 4;
            if suit1 == suit2 {
                push_rate[rank1][rank2] += pusher[1][k];
                call_rate[rank1][rank2] += caller[1][k];
            } else {
                push_rate[rank2][rank1] += pusher[1][k];
                call_rate[rank2][rank1] += caller[1][k];
            }
            overall_push_rate += pusher[1][k];
            overall_call_rate += caller[1][k];
            k += 1;
        }
    }

    for i in 0..13 {
        for j in 0..13 {
            let count = if i == j {
                6.0
            } else if i < j {
                4.0
            } else {
                12.0
            };
            push_rate[i][j] /= count;
            call_rate[i][j] /= count;
        }
    }

    overall_push_rate /= 52.0 * 51.0 / 2.0;
    overall_call_rate /= 52.0 * 51.0 / 2.0;

    println!();
    println!(
        "[Heads-up Push/Fold Hold'em] (effective stack = {}[bb])",
        effective_stack
    );
    println!("- Exploitability: {:+.3e}[bb]", exploitability);

    println!();
    println!("[Pusher (Small blind)]");
    println!("- EV: {:+.4}[bb]", ev);
    println!("- Overall push rate: {:.2}%", 100.0 * overall_push_rate);
    println!(" |   A     K     Q     J     T     9     8     7     6     5     4     3     2");
    println!("-+------------------------------------------------------------------------------");
    for i in 0..13 {
        let rank1 = 12 - i;
        print!(
            "{}|",
            ["2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K", "A"][rank1]
        );
        for j in 0..13 {
            let rank2 = 12 - j;
            if push_rate[rank2][rank1] >= 0.9995 {
                print!(" 100.%");
            } else if push_rate[rank2][rank1] < 0.0005 {
                print!("   -  ");
            } else {
                print!(" {:>4.1}%", 100.0 * push_rate[rank2][rank1]);
            }
        }
        println!();
    }

    println!();
    println!("[Caller (Big blind)]");
    println!("- EV = {:+.4}[bb]", -ev);
    println!("- Overall call rate: {:.2}%", 100.0 * overall_call_rate);
    println!(" |   A     K     Q     J     T     9     8     7     6     5     4     3     2");
    println!("-+------------------------------------------------------------------------------");
    for i in 0..13 {
        let rank1 = 12 - i;
        print!(
            "{}|",
            ["2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K", "A"][rank1]
        );
        for j in 0..13 {
            let rank2 = 12 - j;
            if call_rate[rank2][rank1] >= 0.9995 {
                print!(" 100.%");
            } else if call_rate[rank2][rank1] < 0.0005 {
                print!("   -  ");
            } else {
                print!(" {:>4.1}%", 100.0 * call_rate[rank2][rank1]);
            }
        }
        println!();
    }
}
