mod game;
mod engine;
mod book;

use game::*;
use engine::*;
use book::*;
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::*;

//static mut TRANSPOSITION_TABLE: Lazy<TranspositionTable> = Lazy::new(|| {TranspositionTable::new(23)});
//static mut OPENING_BOOK: Lazy<OpeningBook> = Lazy::new(|| OpeningBook::new());

#[wasm_bindgen]
extern "C" {
    pub fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("Hello, {}!", name));
}

#[wasm_bindgen]
pub fn c4engine(pos: &str) -> i8{
    let moves = pos.chars().filter_map(|c| c.to_digit(10)).map(|d| d as u8);
    let mut table = TranspositionTable::new(20);
    let book = OpeningBook::new();
    let mut game = Game::new();
    for col_num in moves{
        if let (false, _) = game.make_move(col_num){
            return i8::MIN;
        }
    }
    let mut nodes = 0;

    search(&mut game, &mut table, &book, &mut nodes)
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("{:?}", ZOBRIST_TABLE);
    return;

    use std::{thread::sleep, time::{Duration, Instant}};

    let mut game = Game::new();
    let mut table = TranspositionTable::new(23);
    let book = OpeningBook::new();
    let mut nodes = 0;
    
    let start = Instant::now();
    let eval = search(&mut game, &mut table, &book, &mut nodes);
    let time_taken = start.elapsed();
    println!("Eval: {}", eval);
    println!("Time Taken: {:#?}", time_taken);
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

    let book = OpeningBook::new();
    let mut nodes = 0;

    let start = Instant::now();
    for i in 0..1000{
        let mut game = Game::new();
        setup_game(&mut game, &test_moves[i]);
        //let eval = negamax_wrapper(&mut game, 14, &mut transposition_table);
        //let eval = negamax(&mut game, -14, 14, &mut transposition_table, &mut nodes);
        let eval = search(&mut game, &mut transposition_table, &book, &mut nodes);
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
fn trace_pv(game: &mut Game, transposition_table: &mut TranspositionTable, book: &OpeningBook, nodes: &mut u64){
    let mut best_col = 0;
    let mut best_eval = 0;
    game.print();

    while best_eval != -100 {
        best_eval = -100;
        for i in 0..COLS{
            if let (true, row_number) = game.make_move(i){
                let eval = search(game, transposition_table, book, nodes);
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

fn test_book_corrections() {
    let book = OpeningBook::new();
    let mut empty_book = OpeningBook::new();
    for i in 0..BOOK_ENTRIES{
        empty_book.positions[i] = 0;
        empty_book.evals[i] = 0;
    }
    for i in 0..BOOK_ENTRIES{
        if [-689592004,2101158888,1599634104].contains(&book.positions[i]) {
            let mut game = Game::new();
            let (set, p1) = decode(book.positions[i]);
            let mut table = TranspositionTable::new(23);
            game.board_set = set;
            game.board_p1 = p1;
            game.moves_made = 12;
            let mut nodes = 0;
            let eval = search(&mut game, &mut table, &empty_book, &mut nodes);
            println!("pos={}, eval={} engine={}", book.positions[i], book.evals[i], eval);
        }
    }
}

fn test_book_code_decode(){
    let book = OpeningBook::new();
    for i in 0..BOOK_ENTRIES {
        let code = book.positions[i];
        let (set, p1) = decode(code);
        let recode = huffman_code(set, p1, false);
        if code != recode {
            let mut game = Game::new();
            println!("fail");
            game.board_set = set;
            game.board_p1 = p1;
            game.print();
            println!("{code:b}");
            println!("{recode:b}");
        }
    }
    return;
}

fn test_book_lookup(){
    let book = OpeningBook::new();
    for i in 0..BOOK_ENTRIES {
        let code = book.positions[i];
        let (set, p1) = decode(code);
        if let Some(book_lookup) = book.lookup(set, p1){
            if book_lookup != book.evals[i] {
                println!("book_lookup failed code={code} {book_lookup} {}", book.evals[i]);
                panic!();
            }
        } else {
            panic!("book returned None")
        }
    }
}