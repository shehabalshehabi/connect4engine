use std::fmt;
use once_cell::sync::Lazy;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::cmp::Reverse;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Empty,
    Player1,
    Player2,
}
#[derive(PartialEq, Debug)]
pub enum GameStatus {
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

pub static MOVE_ORDER: [u8; 7] = [3,4,2,5,1,6,0];
pub static ROWS: u8 = 6;
pub static COLS: u8 = 7;
pub static COLUMN_MASK: u64 = 2_u64.pow(ROWS as u32)-1;
pub static BOARD_MASK: Lazy<u64> = Lazy::new(|| {
    let mut board_mask = 0;
    for i in 0..COLS {
        board_mask |= COLUMN_MASK << 8 * i
    }
    board_mask
    
});
static BOTTOM_ROW: Lazy<u64> = Lazy::new(|| {
    let mut board_mask = 1;
    for _ in 0..COLS-1 {
        board_mask |= board_mask << 8 
    }
    board_mask
    
});
// Win masks are centered around (3,3) and do go off the board on some edges
pub static WIN_MASK_OFFSET: u8 = 3 * 8 + 3;
pub static WIN_MASKS:[u64;16] = [251658240,
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

pub fn get_bit(board: u64, column_number:u8, row_number:u8) -> bool{
    let index = (column_number << 3) + row_number;
    let mask = 1 << index;
    if board & mask == 0{
        false
    } else {
        true
    }
}

pub fn set_bit(board: u64, column_number:u8, row_number:u8, set: bool)->u64{
    let index = (column_number << 3) + row_number;
    let mask = 1 << index;
    if set {
        board | mask
    } else {
        board & !mask
    }
}

pub fn print_board(board: u64){
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

pub static ZOBRIST_TABLE: Lazy<[[[u64;2]; ROWS as usize]; COLS as usize]> = Lazy::new(|| {
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
pub fn check_board_for_win(board: u64)->bool{
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

pub fn get_winning_squares(player_squares:u64, played: u64)->u64{
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

pub fn stable_sort_moves(col_scores: [(usize, i32);7], playable_cols: usize)->[usize;7]{
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

pub struct Game {
    pub board_set: u64,
    pub board_p1: u64,
    pub player_one_turn : bool,
    pub game_status: GameStatus,
    pub moves_made: i8,
    pub position_hash: u64,
}

impl Game {
    pub fn new() -> Self {
        Self {
            board_set: 0,
            board_p1: 0,
            player_one_turn: true,
            game_status: GameStatus::InProgress,
            moves_made: 0,
            position_hash: 0,
        }
    }

    pub fn set_slot(&mut self, column_number: u8, row_number: u8, value: Slot){
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
    
    pub fn get_slot(&self, column_number: u8, row_number: u8)->Slot{
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

    pub fn print(&self){
        for y in (0..ROWS).rev() {
            for x in 0..COLS{
                print!("{}", self.get_slot(x, y));
            }
            println!();
        }
    }

    pub fn make_move(&mut self, column_number:u8) -> (bool, u8){
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

    pub fn unmake_move(&mut self, column_number:u8, row_number: u8) -> bool{
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
    
    pub fn check_win(&mut self, column_number:u8, row_number:u8) -> bool{
        let board = if self.moves_made % 2 == 1 {
            self.board_set & self.board_p1
        } else {
            self.board_set & !self.board_p1
        };
        
        check_board_for_win(board)
    }

    pub fn get_board_playable(&self)->u64{
        ((self.board_set << 1) | *BOTTOM_ROW) & !(self.board_set) & *BOARD_MASK
    }

    pub fn get_winning_move(&self)->Option<u8>{
        let player_squares = if self.player_one_turn {self.board_set & self.board_p1} else {self.board_set & !self.board_p1};
        let winning_squares = get_winning_squares(player_squares, self.board_set);
        let board_playable = self.get_board_playable();
        if (winning_squares & board_playable) != 0 {
            return Some (1);
        }
        None 
    }

    pub fn get_candidate_moves(&mut self)->[u8;7]{
        // We check for winning moves separately. We get candidate moves by checking for places where we have three tokens in a row
        // that could be extended to four followed by two in a row that can be extended to four
        let board_player = if self.moves_made % 2 == 0 {
            self.board_set & self.board_p1
        } else {
            self.board_set & !self.board_p1
        };

        // Loop through moves and evaluate their potential of being part of a winning sequence.
        let mut col_scores = [(0,0);7];
        let mut playable_cols = 0;
        for col_number in MOVE_ORDER{
            if let (true, row_number) = self.make_move(col_number){
                // The playing player has tried his move so we need the squares of the one whose turn it isn't 
                let player_squares = if self.player_one_turn {!self.board_p1 & self.board_set} else {self.board_p1 & self.board_set};
                let col_score = get_winning_squares(player_squares, self.board_set).count_ones();
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

    pub fn get_hash(&self)->u64{
        self.position_hash
    }
}