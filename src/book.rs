use std::{cmp::Ordering, fs};
use crate::game::{set_bit, get_bit};
pub const BOOK_ENTRIES: usize = 4200899;
pub struct OpeningBook{
    pub positions : Box<[i32; BOOK_ENTRIES]>,
    pub evals: Box<[i8; BOOK_ENTRIES]>,
}

impl OpeningBook {
    pub fn new() -> Self{
        let bytes = fs::read("./opening_book/bookDeepDist.dat").expect("");
        let positions: Vec<i32> = bytes.chunks_exact(5)
        .map(|bytes| i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .collect();
        let positions: Box<[i32; 4200899]> = positions.into_boxed_slice()
            .try_into().expect("could not read book");
        let evals: Vec<i8> = bytes.chunks_exact(5)
            .map(|bytes| {
            let bookeval = bytes[4] as i8; // Convert to i16 to prevent immediate overflow
            if bookeval > 0 {
                15 - (100 - bookeval)/2
            } else if bookeval < 0 {
                - 15 - (-99-bookeval)/2
            } else {
                0
            }
            })
            .collect();

        let mut evals: Box<[i8; 4200899]> = evals.into_boxed_slice()
        .try_into().expect("could not read book");
        for i in 0..BOOK_ENTRIES{
            // Correct opening book issues found by ddrhoardarmer
            // https://github.com/MarkusThill/Connect-Four/issues/3
            match positions[i] {
                -689592004 => {evals[i]=7}
                2101158888 => {evals[i]=4}
                1599634104 => {evals[i]=2}
                _ => {},
            }
        }
        OpeningBook {positions, evals}
    }

    pub fn lookup(&self, board_set:u64, board_p1: u64)->Option<i8> {
        let code = huffman_code(board_set, board_p1, false);
        let eval = self.search(code, 0, BOOK_ENTRIES-1);
        if eval != None{
            return eval;
        }

        let code_reverse = huffman_code(board_set, board_p1, true);
        let eval = self.search(code_reverse, 0, BOOK_ENTRIES-1);
        return eval;
    }

    pub fn search(&self, pos:i32, start:usize, end:usize)->Option<i8>{
        if start > end {
            None
        } else {
            let mid = (start + end) / 2;
            match pos.cmp(&self.positions[mid]){
                Ordering::Less => {self.search(pos, start, mid-1)},
                Ordering::Equal => {Some(self.evals[mid])},
                Ordering::Greater => {self.search(pos, mid+1, end)},
            }
        }
    }
 }

const COL_ORDER: [usize;7]=[0,1,2,3,4,5,6];
const REV_COL_ORDER: [usize;7]=[6,5,4,3,2,1,0];
pub fn huffman_code(board_set:u64, board_p1: u64, reverse: bool)->i32{
    let mut code:i32 = 0;
    let col_order = if reverse {REV_COL_ORDER} else {COL_ORDER};
    for col_num in col_order{
        let mut col_set = board_set >> 8 * col_num;
        let mut col_p1 = board_p1 >> 8 * col_num;
        for _ in 0..7 { // Check 7th row which should never be set to break to next col
            if col_set & 1 == 1{
                code = code << 2;
                if col_p1 & 1 == 1 {
                    code = code | 2;
                } else {
                    code = code | 3;
                }
            } else {
                code = code << 1;
                break;
            }
            col_set = col_set >> 1;
            col_p1 = col_p1 >> 1;
        }
        //println!("decoding col{col_num} {code:b}")
    }
    code << 1
}

pub fn decode(code: i32) -> (u64, u64){
    let mut bit = 31;
    let mut board_set = 0;
    let mut board_p1 = 0;
    let mut col = 0;
    let mut row = 0;
    while (bit >= 0) && (col < 7) {
        if (code >> bit) & 1 == 0 {
            col += 1;
            row = 0;
            bit -= 1
        } else {
            bit -= 1;
            board_set = set_bit(board_set, col, row, true);
            if (code >> bit) & 1 == 0 {
                board_p1 = set_bit(board_p1, col, row, true)
            }
            bit -= 1;
            row += 1;
        }
    }
    (board_set, board_p1)
}