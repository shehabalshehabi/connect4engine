use std::collections::btree_map::Keys;
use std::{cmp::max, fmt, i8};
use std::collections::HashMap;
use rand::{Rng, SeedableRng};
use wasm_bindgen::prelude::*;
use once_cell::sync::Lazy;
use rand::rngs::StdRng;

#[wasm_bindgen]
extern "C" {
    pub fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("Hello, {}!", name));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Empty,
    Player1,
    Player2,
}
#[derive(PartialEq, Debug)]
enum GameStatus {
    InProgress,
    Draw,
    Player1Win,
    Player2Win,
}

impl fmt::Display for Slot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let symbol = match self {
            Slot::Empty => ".",
            Slot::Player1 => "X",
            Slot::Player2 => "O",
        };
        write!(f, "{}", symbol)
    }
}

const ROWS: u8 = 6;
const COLS: u8 = 7;
const MOVE_ORDER: [u8; 7] = [3,4,2,5,1,6,0];

static ZOBRIST_TABLE: Lazy<[[[u64;2]; ROWS as usize]; COLS as usize]> = Lazy::new(|| {
    let mut rng = StdRng::seed_from_u64(0);
    let mut table = [[[0;2]; ROWS as usize]; COLS as usize];
    for x in 0..COLS {
        for y in 0..ROWS {
            for p in 0..2{
                table[x as usize][y as usize][p] = rng.random();
            }
        }
    }
    table
});

struct Game {
    board_set: u64,
    board_p1: u64,
    player_one_turn : bool,
    game_status: GameStatus,
    moves_made: i8,
    position_hash: u64,
}

impl Game {
    fn new() -> Self {
        Self {
            board_set: 0,
            board_p1: 0,
            player_one_turn: true,
            game_status: GameStatus::InProgress,
            moves_made: 0,
            position_hash: 0,
        }
    }

    fn set_slot(&mut self, column_number: u8, row_number: u8, value: Slot){
        // This function doesn't check that a slot hasn't already been assigned to the opponent.
        // It trusts its callers to check before invoking it
        let index = (column_number << 3) + row_number;
        let mask = 1 << index;
        if value == Slot::Empty{
            self.board_set &= !mask;
            return;
        }
        self.board_set |= mask;
        if  value == Slot::Player1 {
            self.board_p1 |= mask;
        } else {
            self.board_p1 &= !mask;
        }
    }
    
    fn get_slot(&self, column_number: u8, row_number: u8)->Slot{
        let index = (column_number << 3) + row_number;
        let mask = 1 << index;
        if (self.board_set & mask== 0){
            Slot::Empty
        } else if (self.board_p1 & mask == 0){
            Slot::Player1
        } else {
            Slot::Player2
        }
    }

    fn print(&self){
        for y in (0..ROWS).rev() {
            for x in 0..COLS{
                print!("{}", self.get_slot(x, y));
            }
            println!();
        }
    }

    fn make_move(&mut self, column_number:u8) -> bool{
        if !(self.game_status == GameStatus::InProgress) {
            return false
        }
        let mut move_made = false;
        for i in 0..ROWS {
            if self.get_slot(column_number, i) == Slot::Empty {
                if self.player_one_turn {
                    self.set_slot(column_number, i, Slot::Player1)
                } else {
                    self.set_slot(column_number, i, Slot::Player2)
                }
                self.moves_made += 1;
                if self.check_win(column_number, i){
                    if self.player_one_turn {
                        self.game_status = GameStatus::Player1Win
                    } else {
                        self.game_status = GameStatus::Player2Win
                    }
                } else if self.moves_made == 42 {
                    self.game_status = GameStatus::Draw
                }
                if self.player_one_turn {
                    self.position_hash ^= ZOBRIST_TABLE[column_number as usize][i as usize][0];
                } else {
                    self.position_hash ^= ZOBRIST_TABLE[column_number as usize][i as usize][1];
                }
                self.player_one_turn = !self.player_one_turn;
                return true;
            }
        }
        false
    }

    fn unmake_move(&mut self, column_number:u8) -> bool{
        // We do not check if this was the last move and leave it to the caller to ensure that it was.
        // We do not eveb check if it was possible for the player whose turn it was last played the move.
        for i in (0..ROWS).rev() {
            if self.get_slot(column_number, i) != Slot::Empty {
                self.set_slot(column_number, i, Slot::Empty);
                self.moves_made -= 1;
                self.game_status = GameStatus::InProgress;
                self.player_one_turn = !self.player_one_turn;
                if self.player_one_turn {
                    self.position_hash ^= ZOBRIST_TABLE[column_number as usize][i as usize][0];
                } else {
                    self.position_hash ^= ZOBRIST_TABLE[column_number as usize][i as usize][1];
                }
                return true
            }
        }
        false
    }
    
    fn check_win(&mut self, column_number:u8, row_number:u8) -> bool{
        let board = if self.moves_made % 2 == 1 {
            self.board_set & self.board_p1
        } else {
            self.board_set & !self.board_p1
        };

        if (board & (board << 1) & (board << 2) & (board << 3)) != 0 {
            return  true;
        }

        if (board & (board << 8) & (board << 16) & (board << 24)) != 0 {
            return  true;
        }

        if (board & (board << 9) & (board << 18) & (board << 27)) != 0 {
            return  true;
        }

        if (board & (board << 7) & (board << 14) & (board << 21)) != 0 {
            return  true;
        }
        false
    }

    fn get_hash(&self)->u64{
        self.position_hash
    }
}

fn negamax(game:&mut Game, alpha: i8, beta: i8, transposition_table: &mut TranspositionTable)->i8{
    let max_possible = (43 - game.moves_made)/2;
    if max_possible < alpha {
        return alpha
    }
    let min_possible = -(42 - game.moves_made)/2;
    if min_possible > beta {
        return  beta;
    }

    match &game.game_status {
        GameStatus::InProgress => (),
        GameStatus::Draw => return 0,
        _ => return -22 + (game.moves_made+1)/2, // negamax can only be called in a decided game by lost player
    }

    let mut new_alpha = alpha;
    let pos = game.get_hash();
    
    
    if let Some(eval) = transposition_table.get(pos){
        match eval.value_type {
            ValueType::Exact => {
                return eval.value;
            }
            ValueType::LowerBound => {
                if eval.value >= beta {
                    return eval.value;
                }
            }
            ValueType::UpperBound => {
                if eval.value <= alpha {
                    return eval.value;
                }
            }
        }
    }

    let mut value = i8::MIN;
    for col_num in MOVE_ORDER {
        if game.make_move(col_num){
            value = max(value, -negamax(game, -beta, -alpha, transposition_table));
            game.unmake_move(col_num);
            new_alpha = max(new_alpha, value);
            if new_alpha > beta {
                break;
            }
        }
    }
    let value_type = if value > beta {
        ValueType::LowerBound
    } else if value < alpha {
        ValueType::UpperBound
    } else {
        ValueType::Exact
    };
    transposition_table.insert(pos, Eval {
        value,
        value_type
    });
    value
}

#[derive(Clone)]
struct Eval{
    value: i8,
    value_type: ValueType,
}

#[derive(PartialEq, Eq, Clone)]
enum ValueType {
    Exact,
    UpperBound,
    LowerBound,
}

#[derive(Clone)]
struct TranspositionTableEntry{
    key: u64,
    eval: Eval,
}

struct TranspositionTable{
    address_mask : u64,
    entries : Box<[Option<TranspositionTableEntry>]>,
}

impl TranspositionTable {
    fn new(n: usize) -> Self {
        Self {
            address_mask: (1<<n)-1,
            entries: vec![None; 1<<n].into_boxed_slice()
        }
    }
    fn insert(&mut self, key: u64, value: Eval){
        let position = key & self.address_mask;
        self.entries[position as usize] = Some(TranspositionTableEntry{
            key,
            eval: value,
        });
    }
    fn get(&mut self, key: u64)->Option<&Eval>{
        let position = key & self.address_mask;
        if let Some(entry) = &self.entries[position as usize]{
            if entry.key == key {
                return Some(&entry.eval)
            }
        }
        None
    }
}

fn negamax_wrapper(game:&mut Game, depth:u8, transposition_table: &mut TranspositionTable)->i8{
    negamax(game, -i8::MAX, i8::MAX, transposition_table) // Need to be able to negate values -128 is i8::MIN and larger than i8::MAX
}

fn search(game: &mut Game, transposition_table: &mut TranspositionTable){
    let mut max = (43 - game.moves_made)/2;
    let mut min = (42 - game.moves_made)/2;

    while (max > min){
        let med = (min + min) / 2;
        let value = negamax(game, med, med+1, transposition_table);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use std::time::Instant;
    use tqdm::tqdm; //Adds a noticable overhead but is satisfying to look at

    let path = "test_cases/Test_L3_R1";
    let (test_moves, test_evals) = read_test_file(path);
    let mut transposition_table = TranspositionTable::new(18);
    println!("mem {}", std::mem::size_of::<TranspositionTableEntry>());

    let start = Instant::now();
    for i in tqdm(0..1000){
        let mut game = Game::new();
        setup_game(&mut game, &test_moves[i]);
        //let eval = negamax_wrapper(&mut game, 14, &mut transposition_table);
        let eval = negamax(&mut game, -1, 1, &mut transposition_table);
        //println!("game {}, eval {}, answer {}", i, eval, test_evals[i]);
        if eval.signum() != test_evals[i].signum(){
            println!("game {}, eval {}, answer {}", i, eval, test_evals[i]);
            panic!("test {i} failed!");
        }
    }
    let time_taken = start.elapsed();
    println!("Time Taken: {:#?}", time_taken);
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
        if !game.make_move(*col_num){
            println!("{:?}", moves);
            println!("game_status: {:#?}, moves_played{}", col_num, game.moves_made);
            panic!("unable to make moves in test case!");
        }
    }
}