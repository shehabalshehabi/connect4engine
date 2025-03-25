mod game;
mod engine;
mod book;

use game::*;
use engine::*;
use book::*;
use wasm_bindgen::prelude::*;


#[wasm_bindgen]
extern "C" {
    pub fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("Hello, {}!", name));
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use std::{thread::sleep, time::{Duration, Instant}};
    use tqdm::tqdm; //Adds a noticable overhead but is satisfying to look at

    let book = OpeningBook::new();
    println!("HERE");
    return;

    /*let mut game = Game::new();
    game.make_move(3);
    let start = Instant::now();
    
    let mut transposition_table = TranspositionTable::new(30);
    let mut nodes = 0;
    println!("{}", search(&mut game, &mut transposition_table, &mut nodes));
    let time_taken = start.elapsed();
    println!("Time Taken: {:#?}", time_taken);
    return;*/


    let path = "test_cases/Test_L3_R1";
    let (test_moves, test_evals) = read_test_file(path);
    let mut transposition_table = TranspositionTable::new(30);
    println!("mem {}", std::mem::size_of::<u64>());

    let mut nodes = 0;

    let start = Instant::now();
    for i in tqdm(0..1000){
        let mut game = Game::new();
        setup_game(&mut game, &test_moves[i]);
        //let eval = negamax_wrapper(&mut game, 14, &mut transposition_table);
        //let eval = negamax(&mut game, -14, 14, &mut transposition_table, &mut nodes);
        let eval = search(&mut game, &mut transposition_table, &mut nodes);
        //println!("game {}, eval {}, answer {}", i, eval, test_evals[i]);
        if eval != test_evals[i]{
            println!("game {}, eval {}, answer {}", i, eval, test_evals[i]);
            panic!("test {i} failed!");
        }
    }
    let time_taken = start.elapsed();
    println!("Mean Time Taken: {:#?}", time_taken/1000);
    println!("Mean Nodes: {:#?}", nodes/1000);
}

#[cfg(not(target_arch = "wasm32"))]
fn trace_pv(game: &mut Game, transposition_table: &mut TranspositionTable, nodes: &mut u64){
    let mut best_col = 0;
    let mut best_eval = 0;
    game.print();

    while best_eval != -100 {
        best_eval = -100;
        for i in 0..COLS{
            if let (true, row_number) = game.make_move(i){
                let eval = search(game, transposition_table, nodes);
                println!("{i}:{} hash={}", -eval, game.get_hash());
                if -eval > best_eval{
                    best_eval = -eval;
                    best_col = i;
                }
                game.unmake_move(i, row_number);
            }
        }
        if best_eval != -100{
            game.make_move(best_col);
            println!("move made: {}", best_col);
            game.print();
            println!()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_test_file(filename: &str)->(Vec<Vec<u8>>,Vec<i8>) {
    use std::{fs::File, io::{BufRead, BufReader}};

    let mut test_moves : Vec<Vec<u8>> = Vec::new();
    let mut test_evals : Vec<i8> = Vec::new();

    let file = File::open(filename).expect("Couldn't find file");
    let file = BufReader::new(file);
    for line in file.lines(){
        let line = line.expect("couldn't read line");
        if let Some((moves, eval)) = line.split_once(" "){
            let moves : Vec<u8> = moves.chars()
                .filter_map(|c| c.to_digit(10).map(|x| (x-1) as u8))
                .collect();
            if let Ok(eval) = eval.parse::<i8>() {
                test_moves.push(moves);
                test_evals.push(eval);
            } else {
                panic!("Couldn't parse eval {}", eval);
            }
        }
    }
    (test_moves, test_evals)
}

#[cfg(not(target_arch = "wasm32"))]
fn setup_game(game:&mut Game, moves: &Vec<u8>){
    for col_num in moves{
        if let (false, _) = game.make_move(*col_num){
            println!("{:?}", moves);
            println!("game_status: {:#?}, moves_played{}", col_num, game.moves_made);
            panic!("unable to make moves in test case!");
        }
    }
}