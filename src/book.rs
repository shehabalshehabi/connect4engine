use std::fs;
const BOOK_ENTRIES: usize = 4200899;
pub struct OpeningBook{
    positions : Box<[u32; BOOK_ENTRIES]>,
    evals: Box<[u8; BOOK_ENTRIES]>,
}

impl OpeningBook {
    pub fn new() -> Self{
        let bytes = fs::read("./opening_book/bookDeepDist.dat").expect("");
        let positions: Vec<u32> = bytes.chunks_exact(5)
        .map(|bytes| u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .collect();
        let positions= positions.into_boxed_slice()
        .try_into().expect("could not read book");
        let evals: Vec<u8> = bytes.chunks_exact(5)
        .map(|bytes| u8::from_be_bytes([bytes[4]]))
        .collect();
        let evals = evals.into_boxed_slice()
        .try_into().expect("could not read book");;
        OpeningBook {positions, evals}
    }
}

const COL_ORDER: [usize;7]=[0,1,2,4,5,6,7];
const REV_COL_ORDER: [usize;7]=[7,6,5,4,3,2,1];
fn huffman_code(board_set:u64, board_p1: u64, reverse: bool)->u64{
    let mut code:u64 = 0;
    let col_order = if reverse {REV_COL_ORDER} else {COL_ORDER};
    for col_num in col_order{
        let mut col_set = board_set >> 8 * col_num;
        let mut col_p1 = board_p1 >> 8 * col_num;
        for _ in 0..7 { // Check 7th row which should never be set to break to next col
            if col_set & 1 == 1{
                code = code << 1;
                if col_p1 & 1 == 1 {
                    code = code | 3;
                } else {
                    code = code | 2;
                }
            } else {
                code = code << 1;
                break;
            }
        }
    }
    code
}