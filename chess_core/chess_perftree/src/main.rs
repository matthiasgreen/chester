use chess_core::{hash::NoopHasher, r#move::MoveList, position::Position};

fn main() {
    let args: Vec<_> = std::env::args().collect::<Vec<_>>();
    let (depth, fen, moves) = match &args[..] {
        [_, depth, fen, moves] => (depth, fen, Some(moves)),
        [_, depth, fen] => (depth, fen, None),
        _ => panic!(),
    };

    perftree(
        depth.parse().unwrap(),
        fen,
        moves.map(|x| x.split_whitespace().collect()),
    );
}

fn perftree(depth: u8, fen: &str, moves: Option<Vec<&str>>) {
    // depth is the maximum depth of the evaluation,
    // fen is the Forsyth-Edwards Notation string of some base position,
    // moves is an optional list of moves from the base position to the position to be evaluated, where each move is formatted as $source$target$promotion, e.g. e2e4 or a7b8Q.

    // The script is expected to output the results of the perft function to standard output, with the following format:
    // For each move available at the current position, print the move and the number of nodes at the given depth which are an ancestor of that move, separated by whitespace.
    // After the list of moves, print a blank line.
    // Finally, print the total node count on its own line.

    let total_nodes = &mut 0;
    let mut position = Position::from_fen(fen, NoopHasher {});

    if let Some(moves) = moves {
        make_move_sequence(&mut position, moves);
    }

    // let start = std::time::Instant::now();
    iter_first_level_moves(&mut position, depth, total_nodes);
    println!();
    println!("{}", *total_nodes);
    // dbg!(start.elapsed());
}

fn make_move_sequence(position: &mut Position<NoopHasher>, moves: Vec<&str>) {
    for m in moves {
        let mut found_move = None;
        let mut move_list = MoveList::new();
        move_list.new_ply();

        position.pseudo_legal_moves(&mut move_list);

        for m2 in move_list.current_ply() {
            if m2.matches_perft_string(m) {
                found_move = Some(m2);
                break;
            }
        }
        if let Some(m) = found_move {
            position.make(*m);
        } else {
            panic!("Move not found");
        }
    }
}

fn iter_first_level_moves(position: &mut Position<NoopHasher>, depth: u8, total_nodes: &mut u64) {
    let mut move_list = MoveList::new();

    position.apply_children(&mut move_list, |m, position, move_list| {
        let count = &mut 0;
        recursive_perft(position, move_list, depth - 1, count);
        println!("{} {}", m, count);
        *total_nodes += *count;
    });
}

fn recursive_perft(
    position: &mut Position<NoopHasher>,
    move_list: &mut MoveList,
    depth: u8,
    nodes: &mut u64,
) {
    if depth == 0 {
        *nodes += 1;
        return;
    }
    move_list.new_ply();
    position.pseudo_legal_moves(move_list);
    let ply_number = move_list.ply_number();
    let ply_size = move_list.ply_size(ply_number);
    for m in 0..ply_size {
        let m = move_list.r#move(ply_number, m);
        position.make(m);
        if position.was_move_legal() {
            if depth == 1 {
                *nodes += 1;
            } else {
                // SearchContext::new(make_unmaker.state, 0).evaluate();
                recursive_perft(position, move_list, depth - 1, nodes);
            }
        }
        position.unmake(m);
    }
    move_list.drop_current_ply();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recursive_perft_test() {
        let initial_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let position_2 = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
        let position_3 = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
        let cases = [
            // initial position
            (initial_fen, 1, 20),
            (initial_fen, 2, 400),
            (initial_fen, 3, 8902),
            (initial_fen, 4, 197281),
            // (initial_fen, 5, 4865609),
            // (initial_fen, 6, 119060324),

            // position 2
            (position_2, 1, 48),
            (position_2, 2, 2039),
            (position_2, 3, 97862),
            // (position_2, 4, 4085603),
            // (position_2, 5, 193690690),

            // position 3
            (position_3, 1, 14),
            (position_3, 2, 191),
            (position_3, 3, 2812),
            (position_3, 4, 43238),
            // (position_3, 5, 674624),
            // (position_3, 6, 11030083),
        ];
        for (fen, depth, nodes) in cases {
            let mut position = Position::from_fen(fen, NoopHasher {});
            dbg!(depth);
            let mut move_list = MoveList::new();
            let mut count = 0;
            recursive_perft(&mut position, &mut move_list, depth, &mut count);
            assert_eq!(count, nodes);
        }
    }
}
