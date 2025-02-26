use std::{cmp::max, fmt, i8};

use wasm_bindgen::prelude::*;

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

const ROWS: usize = 6;
const COLS: usize = 7;

struct Game {
    board: [[Slot; ROWS]; COLS],
    player_one_turn : bool,
    game_status: GameStatus,
    moves_made: i8,
}

impl Game {
    fn new() -> Self {
        Self {
            board: [[Slot::Empty; ROWS]; COLS],
            player_one_turn: true,
            game_status: GameStatus::InProgress,
            moves_made: 0
        }
    }

    fn print(&self){
        for y in (0..ROWS).rev() {
            for x in 0..COLS{
                print!("{}", self.board[x][y]);
            }
            println!();
        }
    }

    fn make_move(&mut self, column_number:usize) -> bool{
        if !(self.game_status == GameStatus::InProgress) {
            return false
        }
        let mut move_made = false;
        for i in 0..ROWS {
            if self.board[column_number][i] == Slot::Empty {
                if self.player_one_turn {
                    self.board[column_number][i] = Slot::Player1
                } else {
                    self.board[column_number][i] = Slot::Player2
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
                self.player_one_turn = !self.player_one_turn;
                return true;
            }
        }
        false
    }

    fn unmake_move(&mut self, column_number:usize) -> bool{
        // We do not check if this was the last move and leave it to the caller to ensure that it was.
        // We do not eveb check if it was possible for the player whose turn it was last played the move.
        for i in (0..ROWS).rev() {
            if self.board[column_number][i] != Slot::Empty {
                self.board[column_number][i] = Slot::Empty;
                self.moves_made -= 1;
                self.game_status = GameStatus::InProgress;
                self.player_one_turn = !self.player_one_turn;
                return true
            }
        }
        false
    }
    
    fn check_win(&mut self, column_number:usize, row_number:usize) -> bool{
        let player = self.board[column_number][row_number];
        if row_number >= 3 {
            for y in row_number-3..row_number+1 {
                if self.board[column_number][y]!= player {
                    break;
                }
                if y == row_number {
                    return true;
                }
            }
        }
        let mut run_length;
        for y_delta_mult in -1..2{
            run_length = 0;
            for delta in -3..4{
                if let (Some(x), Some(y)) = (
                    column_number.checked_add_signed(delta),
                    row_number.checked_add_signed(y_delta_mult * delta)
                ) {
                    if x<COLS && y<ROWS && self.board[x][y] == player {
                        run_length += 1;

                        if run_length >= 4 {
                            return  true;
                        }
                    } else {
                        run_length = 0
                    }
                } else {
                    run_length = 0
                }
            }
        }
        false
    }
}

fn negamax(game:&mut Game, depth:u8, alpha: i8, beta: i8)->i8{
    match &game.game_status {
        GameStatus::InProgress => (),
        GameStatus::Draw => return 0,
        _ => return -22 + (game.moves_made+1)/2, // negamax can only be called in a decided game by lost player
    }

    let mut alpha = alpha;

    if depth == 0 {
        return 0
    } else {
        let mut value = i8::MIN;
        for col_num in 0..COLS {
            if game.make_move(col_num){
                value = max(value, -negamax(game, depth-1, -beta, -alpha));
                game.unmake_move(col_num);
                alpha = max(alpha, value);
                if alpha > beta {
                    break;
                }
            }
        }
        return value
    }
}

fn negamax_wrapper(game:&mut Game, depth:u8)->i8{
    negamax(game, depth, -i8::MAX, i8::MAX) // Need to be able to negate values -128 is i8::MIN and larger than i8::MAX
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {

    let path = "test_cases/Test_L3_R1";
    let (test_moves, test_evals) = read_test_file(path);

    for i in 0..1000{
        let mut game = Game::new();
        setup_game(&mut game, &test_moves[i]);
        let eval = negamax_wrapper(&mut game, 14);
        //println!("game {}, eval {}, answer {}", i, eval, test_evals[i]);
        if eval != test_evals[i]{
            panic!("test {i} failed!");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_test_file(filename: &str)->(Vec<Vec<usize>>,Vec<i8>) {
    use std::{fs::File, io::{BufRead, BufReader}};

    let mut test_moves : Vec<Vec<usize>> = Vec::new();
    let mut test_evals : Vec<i8> = Vec::new();

    let file = File::open(filename).expect("Couldn't find file");
    let file = BufReader::new(file);
    for line in file.lines(){
        let line = line.expect("couldn't read line");
        if let Some((moves, eval)) = line.split_once(" "){
            let moves : Vec<usize> = moves.chars()
                .filter_map(|c| c.to_digit(10).map(|x| (x-1) as usize))
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
fn setup_game(game:&mut Game, moves: &Vec<usize>){
    for col_num in moves{
        if !game.make_move(*col_num){
            println!("{:?}", moves);
            println!("game_status: {:#?}, moves_played{}", col_num, game.moves_made);
            panic!("unable to make moves in test case!");
        }
    }
}