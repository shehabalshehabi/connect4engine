use crate::game::*;
use crate::book::*;
use std::collections::HashSet;
use std::u64;
use std::{cmp::{max, min}, i8};

pub fn negamax(game:&mut Game, alpha: i8, beta: i8, transposition_table: &mut TranspositionTable,
        book: &OpeningBook, nodes: &mut u64)->i8{
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

    let player_slots = if game.player_one_turn{game.board_p1 & game.board_set} else {!game.board_p1 & game.board_set};
    let player_winning_squares = get_winning_squares(player_slots, game.board_set);
    let board_playable = game.get_board_playable();

    if board_playable & player_winning_squares != 0 {
        return max_possible;
    }

    if game.moves_made == 12 {
        if let Some(eval) = book.lookup(game.board_set, game.board_p1){
            return eval;
        }
    }

    let opponent_slots = if game.player_one_turn{!game.board_p1 & game.board_set} else {game.board_p1 & game.board_set};
    let opponent_winning_squares = get_winning_squares(opponent_slots, game.board_set);

    match (board_playable & opponent_winning_squares).count_ones() {
        0 => (),
        1 => {
            for i in 0..COLS{
                if board_playable & opponent_winning_squares & (COLUMN_MASK << (i * 8)) != 0{
                    if let (true, row_number) = game.make_move(i){
                        let val = -negamax(game, -beta, -alpha, transposition_table, book, nodes);
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
            value = max(value, -negamax(game, -beta, -alpha, transposition_table, book, nodes));
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
pub struct Eval{
    value: i8,
    value_type: ValueType,
}

#[derive(PartialEq, Eq, Clone)]
pub enum ValueType {
    Exact,
    UpperBound,
    LowerBound,
}

#[derive(Clone)]
pub struct TranspositionTableEntry{
    key: u64,
    eval: Eval,
}

pub struct TranspositionTable{
    address_mask : u64,
    entries : Box<[u64]>,
}

impl TranspositionTable {
    // 64 bit entries. 56 bits for key. Last 8 bits for value. Eval +50 for upper bound -50 for lower bound.
    pub fn new(n: usize) -> Self {
        Self {
            address_mask: (1<<n)-1,
            entries: vec![0; 1<<n].into_boxed_slice()
        }
    }
    pub fn insert(&mut self, key: u64, value: Eval){
        let position = key & self.address_mask;
        let entry_val = match value.value_type {
            ValueType::LowerBound => value.value - 50,
            ValueType::UpperBound => value.value + 50,
            ValueType::Exact => value.value,
        };
        let entry = key >> 8 << 8 | (entry_val as u8 as u64);
        self.entries[position as usize] = entry;
    }
    pub fn get(&mut self, key: u64)->Option<Eval>{
        if key==0 {
            return None;
        }
        let position = key & self.address_mask;
        let entry =  self.entries[position as usize];
        if key >> 8 == entry >> 8 {
            let entry_val = entry as i8;
            if entry_val < -25 {
                return Some(Eval{
                    value: entry_val + 50,
                    value_type: ValueType::LowerBound,
                });
            }
            if entry_val > 25 {
                return Some(Eval{
                    value: entry_val - 50,
                    value_type: ValueType::UpperBound,
                });
            }
            return Some(Eval{
                value: entry_val,
                value_type: ValueType::Exact,
            });
        }
        None
    }
}

pub fn negamax_wrapper(game:&mut Game, transposition_table: &mut TranspositionTable)->i8{
    //negamax(game, -i8::MAX, i8::MAX, transposition_table) // Need to be able to negate values -128 is i8::MIN and larger than i8::MAX
    0
}

pub fn search(game: &mut Game, transposition_table: &mut TranspositionTable, book: &OpeningBook, nodes: &mut u64)->i8{
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
        let result = negamax(game, window, window+1, transposition_table, book, nodes);
        //println!("{minimum_possible}, {maximum_possible}, {window}, {result}");
        if result <= window {
            maximum_possible = window
        } else {
            minimum_possible = window + 1
        }
    }
    minimum_possible
}

pub fn calculate_tree_width(game:&mut Game, plies: i8, seen: &mut HashSet<u64>)->u64{
    if game.moves_made == plies {
        1
    } else {
        let mut positions = 0;

        for col_number in 0..COLS{
            if let (true, row_number) = game.make_move(col_number){
                let pos = game.get_hash();
                if seen.contains(&pos){
                    game.unmake_move(col_number, row_number);
                    continue;
                }
                else {
                    seen.insert(pos);
                    print!("{col_number}");
                    positions += calculate_tree_width(game, plies, seen);
                    game.unmake_move(col_number, row_number);
                    print!("\x08 \x08");
                }

            }
        }
        positions
    }
}