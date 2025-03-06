use core::borrow;
use std::collections::btree_map::Keys;
use std::u64;
use std::{cmp::{max, min}, fmt, i8};
use std::collections::HashMap;
use rand::{Rng, SeedableRng};
use wasm_bindgen::prelude::*;
use once_cell::sync::Lazy;
use rand::rngs::StdRng;
use std::cmp::Reverse;

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
const COLUMN_MASK: u64 = 2_u64.pow(ROWS as u32)-1;
const BOARD_MASK: Lazy<u64> = Lazy::new(|| {
    let mut board_mask = COLUMN_MASK;
    for i in 1..COLS {
        board_mask = board_mask | (board_mask << 8 * i) 
    }
    board_mask
});
// Win masks are centered around (3,3) and do go off the board on some edges
const WIN_MASKS: Lazy<Vec<u64>> = Lazy::new(|| {
    let mut masks: Vec<u64> = Vec::new();
    // Vertical win masks
    for start in 0..4{
        let mut mask = 0;
        for i in start..start+4{
            mask = set_bit(mask, 3, i, true)
        }
        masks.push(mask);
    }
    // Other win masks
    for y_delta in -1..2{
        for start_offset in -3..1{
            let mut mask = 0;
            for offset in start_offset..start_offset+4{
                let x:u8 = (3 + offset) as u8;
                let y:u8 = (3 + offset * y_delta) as u8;
                mask = set_bit(mask, x, y, true);
            }
            masks.push(mask);
        }
    }
    masks
});
const WIN_MASK_OFFSET: u8 = 3 * 8 + 3;

const WIN_MASK_ARR: Lazy<Vec<Vec<Vec<u64>>>> = Lazy::new(|| {
    let mut mask_arr: Vec<Vec<Vec<u64>>> = Vec::new();
    for col_num in 0..COLS{
        let mut row_masks = Vec::new();
        for row_num in 0..ROWS{
            let mut masks = Vec::new();
            // Vertical win masks
            for start in (if row_num>=3{row_num-3}else{0})..row_num+1{
                let mut mask = 0;
                if start+3>= ROWS{
                    break;
                }
                for y in start..start+4{
                    mask = set_bit(mask, col_num, y, true)
                }
                masks.push(mask);
            }
            
            //Other win masks
            for y_delta in -1..2{
                for start_offset in -3..1{
                    let mut mask = 0;
                    for offset in start_offset..start_offset+4{
                        let x:i8 = col_num as i8 + offset;
                        let y:i8 = row_num as i8 + offset * y_delta;
                        if (x < 0) | (x >= COLS as i8) | (y < 0) | (y >= ROWS as i8){
                            break;
                        }
                        mask = set_bit(mask, x as u8, y as u8, true);
                        if offset == start_offset + 3{
                            masks.push(mask);
                        }
                    }
                }
            }
            //println!("{col_num},{row_num}, {}", masks.len());
            row_masks.push(masks);
        }
        mask_arr.push(row_masks);
    }
    
    mask_arr
});

fn get_bit(board: u64, column_number:u8, row_number:u8) -> bool{
    let index = (column_number << 3) + row_number;
    let mask = 1 << index;
    if (board & mask == 0){
        false
    } else {
        true
    }
}

fn set_bit(board: u64, column_number:u8, row_number:u8, set: bool)->u64{
    let index = (column_number << 3) + row_number;
    let mask = 1 << index;
    if set {
        board | mask
    } else {
        board & !mask
    }
}

fn print_board(board: u64){
    for y in (0..ROWS).rev() {
        for x in 0..COLS{
            if get_bit(board, x, y){
                print!("X")
            } else {
                print!("O")
            }
        }
        println!();
    }
}

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

#[inline(always)]
fn check_board_for_win(board: u64)->bool{
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

    fn make_move(&mut self, column_number:u8) -> (bool, u8){
        if !(self.game_status == GameStatus::InProgress) {
            return (false, 0)
        }
        // Check if column has been played at all - otherwise find slot using bitshift
        let (slot, row_number) = if ((self.board_set >> (8 * column_number)) & 1) == 0 {
            (1<<(8*column_number), 0)
        } else {
            let slot = ((self.board_set << 1) & (COLUMN_MASK << 8 * column_number) | (self.board_set)) - self.board_set;
            //println!("slot 2: {slot}");
            let row_number = match (slot >> 8 * column_number) {
                2=>1,
                4=>2,
                8=>3,
                16=>4,
                32=>5,
                _=>return (false, 0),
            };
            (slot, row_number)
        };
        self.board_set |= slot;
        if self.player_one_turn {
            self.board_p1 |= slot;
        } else {
            self.board_p1 &= !slot;
        }
        if self.player_one_turn {
            self.position_hash ^= ZOBRIST_TABLE[column_number as usize][row_number as usize][0];
        } else {
            self.position_hash ^= ZOBRIST_TABLE[column_number as usize][row_number as usize][1];
        }
        self.moves_made += 1;
        if self.check_win(column_number, row_number){
            if self.player_one_turn {
                self.game_status = GameStatus::Player1Win
            } else {
                self.game_status = GameStatus::Player2Win
            }
        } else if self.moves_made == 42 {
            self.game_status = GameStatus::Draw
        }
        self.player_one_turn = !self.player_one_turn;
        (true, row_number)
    }

    fn unmake_move(&mut self, column_number:u8, row_number: u8) -> bool{
        // We do not check if this was the last move and leave it to the caller to ensure that it was.
        // We do not eveb check if it was possible for the player whose turn it was last played the move.
        let slot = (self.board_set & !(self.board_set >> 1)) & (COLUMN_MASK << 8 * column_number);
        if slot == 0 {
            false
        } else {
            self.board_set &= !slot;
            self.moves_made -= 1;
            self.game_status = GameStatus::InProgress;
            self.player_one_turn = !self.player_one_turn;
            if self.player_one_turn {
                self.position_hash ^= ZOBRIST_TABLE[column_number as usize][row_number as usize][0];
            } else {
                self.position_hash ^= ZOBRIST_TABLE[column_number as usize][row_number as usize][1];
            }
            true
        }
    }
    
    fn check_win(&mut self, column_number:u8, row_number:u8) -> bool{
        let board = if self.moves_made % 2 == 1 {
            self.board_set & self.board_p1
        } else {
            self.board_set & !self.board_p1
        };
        
        check_board_for_win(board)
    }

    fn get_winning_move(&self)->Option<u8>{
        let board_player = if self.moves_made % 2 == 0 {
            self.board_set & self.board_p1
        } else {
            self.board_set & !self.board_p1
        };

        let board_playable = (self.board_set << 1) & !(self.board_set);
        
        for i in 0..COLS{
            let board = board_player | (board_playable & (COLUMN_MASK << 8 * i));
            if check_board_for_win(board){
                return Some(i);
            }
        }
        None
    }

    fn get_candidate_moves(&mut self)->Vec<u8>{
        // We check for winning moves separately. We get candidate moves by checking for places where we have three tokens in a row
        // that could be extended to four followed by two in a row that can be extended to four
        let board_player = if self.moves_made % 2 == 0 {
            self.board_set & self.board_p1
        } else {
            self.board_set & !self.board_p1
        };
        let board_playable= board_player | !self.board_set;

        // Loop through moves and evaluate their potential of being part of a winning sequence.
        let mut col_scores: Vec<(u8, u32)> = Vec::new();
        for col_number in 0..COLS{
            if let (true, row_number) = self.make_move(col_number){
                let mut COL_SCORE = 0;
                for mask in &*WIN_MASK_ARR[col_number as usize][row_number as usize]{
                    if (board_playable & mask).count_ones() == 4{
                        let count = (board_player&mask).count_ones();
                        let mask_score = match count {
                            3 => 257, // 16 * 16 + 1
                            2 => 17, // 16 win masks
                            _ => 1,
                        };
                        COL_SCORE += mask_score; 
                    }                    
                }
                col_scores.push((col_number, COL_SCORE));
                self.unmake_move(col_number, row_number);
            }
        }

        col_scores.sort_by_key(|&(_, score)| Reverse(score));

        return col_scores.iter().map(|&(col_number,_)|col_number).collect();
    }

    fn get_hash(&self)->u64{
        self.position_hash
    }
}

fn negamax(game:&mut Game, alpha: i8, beta: i8, transposition_table: &mut TranspositionTable, nodes: &mut u64)->i8{
    *nodes += 1;
    let max_possible = (43 - game.moves_made)/2;
    if max_possible < alpha {
        return alpha
    }
    let min_possible = -(42 - game.moves_made)/2;
    if min_possible > beta {
        return beta;
    }

    match &game.game_status {
        GameStatus::InProgress => (),
        GameStatus::Draw => return 0,
        _ => return -22 + (game.moves_made+1)/2, // negamax can only be called in a decided game by lost player
    }

    if let Some(_move) = game.get_winning_move(){
        return max_possible
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
    let move_order = game.get_candidate_moves();
    for col_num in move_order {
        if let (true, row_number) = game.make_move(col_num){
            value = max(value, -negamax(game, -beta, -alpha, transposition_table, nodes));
            game.unmake_move(col_num, row_number);
            new_alpha = max(new_alpha, value);
            if new_alpha > beta {
                break;
            }
        }
    }
    // We never get exact values from a null window search and so don't check to speed things up.
    let value_type = if value >= beta {
        ValueType::LowerBound
    } else {
        ValueType::UpperBound
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

fn negamax_wrapper(game:&mut Game, transposition_table: &mut TranspositionTable)->i8{
    //negamax(game, -i8::MAX, i8::MAX, transposition_table) // Need to be able to negate values -128 is i8::MIN and larger than i8::MAX
    0
}

fn search(game: &mut Game, transposition_table: &mut TranspositionTable, nodes: &mut u64)->i8{
    let mut maximum_possible = (42 - game.moves_made)/2;
    let mut minimum_possible = -(43 - game.moves_made)/2;

    /* Iterative deepening algorithm used by Pascal Pons

    We prefer searching higher and lower than the midpoint so we get more pruning of the
    search tree. Our window also defines the depth of our search as we score better for
    earlier wins.
    */

    while (minimum_possible < maximum_possible){
        let mut window = minimum_possible + (maximum_possible-minimum_possible) / 2;
        if (window >= 0) & (maximum_possible/2 > window){
            window = max(window, maximum_possible/2);
        } else if (window <= 0) & (minimum_possible/2 < window){
            window = min(window, minimum_possible/2);
        }
        //println!("{minimum_possible}, {maximum_possible}, {window}");
        let result = negamax(game, window, window+1, transposition_table, nodes);
        //println!("{minimum_possible}, {maximum_possible}, {window}, {result}");
        if result <= window {
            maximum_possible = window
        } else {
            minimum_possible = window + 1
        }
    }
    minimum_possible
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use std::{thread::sleep, time::{Duration, Instant}};
    use tqdm::tqdm; //Adds a noticable overhead but is satisfying to look at

    let path = "test_cases/Test_L2_R1";
    let (test_moves, test_evals) = read_test_file(path);
    let mut transposition_table = TranspositionTable::new(18);
    println!("mem {}", std::mem::size_of::<TranspositionTableEntry>());

    let mut nodes = 0;

    let start = Instant::now();
    for i in tqdm(0..1000){
        let mut game = Game::new();
        setup_game(&mut game, &test_moves[i]);
        //let eval = negamax_wrapper(&mut game, 14, &mut transposition_table);
        //let eval = negamax(&mut game, -1, 1, &mut transposition_table);
        let eval = search(&mut game, &mut transposition_table, &mut nodes);
        //println!("game {}, eval {}, answer {}", i, eval, test_evals[i]);
        if eval != test_evals[i]{
            println!("game {}, eval {}, answer {}", i, eval, test_evals[i]);
            panic!("test {i} failed!");
        }
    }
    let time_taken = start.elapsed();
    println!("Time Taken: {:#?}", time_taken);
    println!("Nodes: {:#?}", nodes);
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