use core::borrow;
use std::collections::btree_map::Keys;
use std::{i32, u64};
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

static ROWS: u8 = 6;
static COLS: u8 = 7;
static MOVE_ORDER: [u8; 7] = [3,4,2,5,1,6,0];
static COLUMN_MASK: u64 = 2_u64.pow(ROWS as u32)-1;
static BOARD_MASK: Lazy<u64> = Lazy::new(|| {
    let mut board_mask = 0;
    for i in 0..COLS {
        board_mask |= (COLUMN_MASK << 8 * i) 
    }
    board_mask
    
});
static BOTTOM_ROW: Lazy<u64> = Lazy::new(|| {
    let mut board_mask = 1;
    for i in 0..COLS-1 {
        board_mask |= board_mask << 8 
    }
    board_mask
    
});
// Win masks are centered around (3,3) and do go off the board on some edges
/*
static WIN_MASKS: Lazy<[u64; 16]> = Lazy::new(|| {
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
    masks.try_into().expect("Error generating win masks")
});*/
static WIN_MASK_OFFSET: u8 = 3 * 8 + 3;

/*static WIN_MASK_ARR: Lazy<Vec<Vec<Vec<u64>>>> = Lazy::new(|| {
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
});*/

const WIN_MASKS:[u64;16] = [251658240,
                            503316480,
                            1006632960,
                            2013265920,
                            135274560,
                            17315143680,
                            2216338391040,
                            283691314053120,
                            134744072,
                            34494482432,
                            8830587502592,
                            2260630400663552,
                            134480385,
                            68853957120,
                            35253226045440,
                            18049651735265280];

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

fn get_winning_squares(player_squares:u64, played: u64)->u64{
    let mut winning_squares = 0;
    winning_squares |= (player_squares << 1) & (player_squares << 2) & (player_squares << 3);

    winning_squares |= (player_squares << 8) & (player_squares << 16) & (player_squares << 24);
    winning_squares |= (player_squares >> 8) & (player_squares << 8) & (player_squares << 16);
    winning_squares |= (player_squares >> 16) & (player_squares >> 8) & (player_squares << 8);
    winning_squares |= (player_squares >> 24) & (player_squares >> 16) & (player_squares >> 8);

    winning_squares |= (player_squares << 9) & (player_squares << 18) & (player_squares << 27);
    winning_squares |= (player_squares >> 9) & (player_squares << 9) & (player_squares << 18);
    winning_squares |= (player_squares >> 18) & (player_squares >> 9) & (player_squares << 9);
    winning_squares |= (player_squares >> 27) & (player_squares >> 18) & (player_squares >> 9);

    winning_squares |= (player_squares << 7) & (player_squares << 14) & (player_squares << 21);
    winning_squares |= (player_squares >> 7) & (player_squares << 7) & (player_squares << 14);
    winning_squares |= (player_squares >> 14) & (player_squares >> 7) & (player_squares << 7);
    winning_squares |= (player_squares >> 21) & (player_squares >> 14) & (player_squares >> 7);

    winning_squares & !played & *BOARD_MASK
}

fn stable_sort_moves(col_scores: [(usize, i32);7], playable_cols: usize)->[usize;7]{
    let mut move_order = [255;7];
    let mut best_score_index;
    let mut best_score;
    let mut prev_best_score = i32::MAX;
    let mut prev_score_index = 0;
    for i in 0..playable_cols{
        best_score = -1;
        best_score_index = 7;
        for j in 0..playable_cols{
            let j_score = col_scores[j].1;
            
            if j_score > prev_best_score {
                continue;
            }
            if (j_score == prev_best_score) & (j <= prev_score_index){
                continue;
            }
            
            if j_score > best_score{
                best_score = j_score;
                best_score_index = j;
            }
        }
        move_order[i] = col_scores[best_score_index].0;
        prev_score_index = best_score_index;
        prev_best_score = best_score;
    }
    move_order
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
        } else if (!self.board_p1 & mask == 0){
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

    fn get_board_playable(&self)->u64{
        ((self.board_set << 1) | *BOTTOM_ROW) & !(self.board_set) & *BOARD_MASK
    }

    fn get_winning_move(&self)->Option<u8>{
        let player_squares = if self.player_one_turn {self.board_set & self.board_p1} else {self.board_set & !self.board_p1};
        let winning_squares = get_winning_squares(player_squares, self.board_set);
        let board_playable = self.get_board_playable();
        if (winning_squares & board_playable) != 0 {
            return Some (1);
        }
        None 
    }

    fn get_candidate_moves(&mut self)->[u8;7]{
        // We check for winning moves separately. We get candidate moves by checking for places where we have three tokens in a row
        // that could be extended to four followed by two in a row that can be extended to four
        let board_player = if self.moves_made % 2 == 0 {
            self.board_set & self.board_p1
        } else {
            self.board_set & !self.board_p1
        };
        let board_playable= board_player | !self.board_set;

        // Loop through moves and evaluate their potential of being part of a winning sequence.
        let mut col_scores = [(0,0);7];
        let mut playable_cols = 0;
        for col_number in 0..COLS{
            if let (true, row_number) = self.make_move(col_number){
                let index = col_number * 8 + row_number;
                // We align the board to the masks
                let (board_playable, board_player) = if (index >= WIN_MASK_OFFSET){
                    let offset = index - WIN_MASK_OFFSET;
                    (board_playable >> offset, board_player >> offset)
                } else {
                    let offset = WIN_MASK_OFFSET - index;
                    (board_playable << offset, board_player << offset)
                };
                let mut col_score = 0;
                for mask in WIN_MASKS{
                    if (board_playable & mask).count_ones() == 4{
                        let count = (board_player&mask).count_ones();
                        let mask_score = match count {
                            3 => 257, // 16 * 16 + 1
                            2 => 17, // 16 win masks
                            _ => 1,
                        };
                        col_score += mask_score; 
                    }                    
                }
                col_scores[playable_cols] = (col_number,col_score);
                playable_cols += 1;
                self.unmake_move(col_number, row_number);
            }
        }

        col_scores[..playable_cols].sort_by_key(|&(_, score)| Reverse(score));
        let mut move_order = [255;7];
        for i in 0..playable_cols{
            move_order[i] = col_scores[i].0
        }
        move_order
    }

    fn get_hash(&self)->u64{
        self.position_hash
    }
}

fn negamax(game:&mut Game, alpha: i8, beta: i8, transposition_table: &mut TranspositionTable, nodes: &mut u64)->i8{
    *nodes += 1;

    match &game.game_status {
        GameStatus::InProgress => (),
        GameStatus::Draw => return 0,
        _ => return -22 + (game.moves_made+1)/2, // negamax can only be called in a decided game by lost player
    }

    let max_possible = 21 - game.moves_made/2;
    if max_possible <= alpha {
        return max_possible
    }
    let min_possible = -21 + (game.moves_made+1)/2;
    if min_possible >= beta {
        return min_possible;
    }

    if let Some(_move) = game.get_winning_move(){
        return max_possible
    }

    let opponent_slots = if game.player_one_turn{!game.board_p1 & game.board_set} else {game.board_p1 & game.board_set};
    let opponent_winning_squares = get_winning_squares(opponent_slots, game.board_set);
    let board_playable = game.get_board_playable();

    match (board_playable & opponent_winning_squares).count_ones() {
        0 => (),
        1 => {
            for i in 0..COLS{
                if board_playable & opponent_winning_squares & (COLUMN_MASK << (i * 8)) != 0{
                    if let (true, row_number) = game.make_move(i){
                        let val = -negamax(game, -beta, -alpha, transposition_table, nodes);
                        game.unmake_move(i, row_number);
                        return val;
                    }
                }
            }
        },
        _ => return min_possible
    }


    let mut alpha = alpha;
    let mut beta = beta;
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
                if eval.value > alpha{
                    alpha = eval.value
                }
            }
            ValueType::UpperBound => {
                if eval.value <= alpha {
                    return eval.value;
                }
                if eval.value < beta {
                    beta = eval.value;
                }
            }
        }
    }

    
    let mut value = i8::MIN;
    let move_order = game.get_candidate_moves();
    for col_num in move_order {
        if col_num == 255{
            break;
        }
        if let (true, row_number) = game.make_move(col_num){
            value = max(value, -negamax(game, -beta, -alpha, transposition_table, nodes));
            game.unmake_move(col_num, row_number);
            alpha = max(alpha, value);
            if alpha >= beta {
                transposition_table.insert(pos, Eval {
                    value: beta,
                    value_type: ValueType::LowerBound,
                });
                return beta;
            }
        }
    }
    transposition_table.insert(pos, Eval {
        value: alpha,
        value_type: ValueType::UpperBound
    });
    alpha
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
    let mut maximum_possible = 21 - game.moves_made/2;
    let mut minimum_possible = -21 + (game.moves_made+1)/2;

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

    let path = "test_cases/Test_L2_R2";
    let (test_moves, test_evals) = read_test_file(path);
    let mut transposition_table = TranspositionTable::new(23);
    println!("mem {}", std::mem::size_of::<TranspositionTableEntry>());

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