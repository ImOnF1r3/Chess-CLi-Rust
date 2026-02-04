use crossterm::{
    style::{Color, Stylize},
    terminal::size,
};
use pleco::Board;
use std::io::Read;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const BACKGROUND1: Color = Color::Rgb {
    r: 100,
    g: 58,
    b: 113,
};
const BACKGROUND2: Color = Color::Rgb {
    r: 139,
    g: 95,
    b: 191,
};
const TEXT1: Color = Color::Rgb {
    r: 224,
    g: 225,
    b: 221,
};
const TEXT2: Color = Color::Rgb {
    r: 13,
    g: 27,
    b: 42,
};
const RED_BACKGROUND: Color = Color::Rgb {
    r: 255,
    g: 71,
    b: 71,
};
const ERROR_TEXT: Color = Color::Rgb { r: 255, g: 0, b: 0 };
const SUCCESS_TEXT: Color = Color::Rgb { r: 0, g: 255, b: 0 };

fn clear_console() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().expect("Errore nel flush");
}

fn get_terminal_size() -> std::io::Result<(u16, u16)> {
    let (width, height) = size()?;
    Ok((width, height))
}

fn get_input(prompt: &str) -> String {
    let mut input = String::new();
    print!("{}", prompt);
    io::stdout().flush().expect("Errore nel flush dell'output");

    io::stdin()
        .read_line(&mut input)
        .expect("Errore nella lettura dell'input");

    let mut trimmed_input = input.trim().to_string();

    // Converti automaticamente formato corto in formato lungo SOLO se sembra una mossa
    if trimmed_input.len() == 4 
        && trimmed_input.chars().all(|c| c.is_alphanumeric()) 
        && trimmed_input.chars().nth(0).unwrap().is_ascii_lowercase()
        && trimmed_input.chars().nth(1).unwrap().is_ascii_digit() {
        trimmed_input.insert(2, '-');
    }

    trimmed_input
}

fn reset_word(matrix: &mut [[i32; 8]; 8]) {
    for x in 0..8 {
        for y in 0..8 {
            if (x == 0 || x == 7) && (y == 0 || y == 7) {
                matrix[x][y] = if x == 0 { 41 } else { 40 }; //Rook
            }
            if (x == 0 || x == 7) && (y == 1 || y == 6) {
                matrix[x][y] = if x == 0 { 31 } else { 30 }; //Horse
            }
            if (x == 0 || x == 7) && (y == 2 || y == 5) {
                matrix[x][y] = if x == 0 { 21 } else { 20 }; //Bishop
            }
            if x == 1 {
                matrix[x][y] = 11; // Pawn
            }
            if x == 6 {
                matrix[x][y] = 10;
            }
            if (x == 0 || x == 7) && (y == 3) {
                matrix[x][y] = if x == 0 { 51 } else { 50 }; //Queen
            }
            if (x == 0 || x == 7) && (y == 4) {
                matrix[x][y] = if x == 0 { 61 } else { 60 }; //King
            }
        }
    }
}

fn spacex_off(screenx: u16, offset: Option<i32>) {
    let offset_value = offset.unwrap_or(0);
    for _xx in 0..((screenx / 2) as i32 - 9 + offset_value) {
        print!(" ");
    }
}

fn spacey(screeny: u16) {
    for _yy in 0..((screeny / 2) - 5) {
        println!(" ");
    }
}

fn fx_stamp_word(matrix: &mut [[i32; 8]; 8], gm: &mut i32, screenx: u16) {
    fn print_piece(num: i32, text_color: Color, background_color: Color) {
        let piece_code = match num {
            1 => "\u{f0859}", // Pedone
            2 => "\u{f085c}", // Alfiere
            3 => "\u{f0858}", // Cavallo
            4 => "\u{f085b}", // Torre
            5 => "\u{f085a}", // Regina
            6 => "\u{f0857}", // Re
            _ => "xx",        // Errore
        };
        print!(
            "{}",
            format!("{} ", piece_code)
                .with(text_color)
                .on(background_color)
        );
    }

    for x in 0..8 {
        spacex_off(screenx, None);
        print!("{} ", 8 - x);
        for y in 0..8 {
            let num: i32 = matrix[x][y];

            let mut background_color = if (x + y) % 2 == 0 {
                BACKGROUND2
            } else {
                BACKGROUND1
            };

            if (num == 60 && *gm % 2 == 0 || num == 61 && *gm % 2 == 1)
                && !check_king(-1, -1, -1, -1, *gm, matrix)
            {
                background_color = RED_BACKGROUND;
            }

            if num == 0 {
                print!("{}", "  ".on(background_color));
            } else {
                let text_color = if num % 2 == 0 { TEXT1 } else { TEXT2 };
                let piece_num = (num - (num % 2)) / 10;
                print_piece(piece_num, text_color, background_color);
            }
        }
        println!();
    }
    spacex_off(screenx, None);
    println!("  A B C D E F G H");
}

fn pz_move(
    string: &str,
    gm: &mut i32,
    matrix: &mut [[i32; 8]; 8],
    black_piece: &mut [i32; 16],
    white_piece: &mut [i32; 16],
    en_passant_row: &mut i32,
    en_passant_col: &mut i32,
) -> bool {
    let input = string.trim();
    let clean_input = input.replace("-", "");

    if clean_input.len() == 4 {
        let chars: Vec<char> = clean_input.chars().collect();

        if let (Some(s_c), Some(s_r), Some(e_c), Some(e_r)) = (
            col_to_number(chars[0]),
            row_to_number(chars[1]),
            col_to_number(chars[2]),
            row_to_number(chars[3]),
        ) {
            let s_c: i32 = s_c as i32;
            let s_r: i32 = 7 - s_r as i32;
            let e_c: i32 = e_c as i32;
            let e_r: i32 = 7 - e_r as i32;

            let m_c: i32 = e_c - s_c;
            let m_r: i32 = e_r - s_r;

            let id_piece: i32 = matrix[s_r as usize][s_c as usize];
            let id_enemy: i32 = matrix[e_r as usize][e_c as usize];

            let mut check: bool = check_piece(matrix, id_piece, m_c, m_r, s_r, s_c, id_enemy);

            // Check for en passant
            let is_en_passant = (id_piece == 10 || id_piece == 11)
                && m_c.abs() == 1
                && id_enemy == 0
                && e_r == *en_passant_row
                && e_c == *en_passant_col;

            if is_en_passant {
                check = true; // Allow the en passant move
            }

            if check_king(s_r, s_c, e_r, e_c, *gm, matrix)
                && (check_eat(matrix, s_r, s_c, e_r, e_c) || is_en_passant)
                && is_valid_turn(id_piece, *gm)
                && check
            {
                if is_en_passant {
                    let captured_pawn = matrix[s_r as usize][e_c as usize];
                    matrix[s_r as usize][e_c as usize] = 0;
                    if id_piece % 2 == 0 {
                        capture_piece(black_piece, captured_pawn);
                    } else {
                        capture_piece(white_piece, captured_pawn);
                    }
                    println!("{}", "En passant capture executed!".with(SUCCESS_TEXT));
                } else if id_enemy != 0 {
                    if id_piece % 2 == 0 {
                        capture_piece(black_piece, id_enemy);
                    } else {
                        capture_piece(white_piece, id_enemy);
                    }
                }

                matrix[e_r as usize][e_c as usize] = matrix[s_r as usize][s_c as usize];
                matrix[s_r as usize][s_c as usize] = 0;
                
                if (id_piece == 60 || id_piece == 61) && m_c.abs() == 2 {
                    let row = e_r as usize;
                    if e_c == 6 { // Arrocco Corto (lato G)
                        matrix[row][5] = matrix[row][7]; // Muove Torre da H a F
                        matrix[row][7] = 0;
                    } else if e_c == 2 { // Arrocco Lungo (lato C)
                        matrix[row][3] = matrix[row][0]; // Muove Torre da A a D
                        matrix[row][0] = 0;
                    }
                }

                if (id_piece == 10 || id_piece == 11) && (s_r - e_r).abs() == 2 {
                    *en_passant_row = (s_r + e_r) / 2;
                    *en_passant_col = s_c;
                    println!(
                        "{}",
                        format!(
                            "En passant opportunity set at row {} col {}",
                            *en_passant_row, *en_passant_col
                        )
                        .with(SUCCESS_TEXT)
                    );
                } else {
                    *en_passant_row = -1;
                    *en_passant_col = -1;
                }

                if (id_piece == 10 || id_piece == 11) && (e_r == 0 || e_r == 7) {
                    touchdown(e_c, e_r, matrix, *gm);
                }

                *gm += 1;

                println!(
                    "{}",
                    format!("Pezzo mosso da ({}, {}) a ({}, {})", s_c, s_r, e_c, e_r)
                        .with(SUCCESS_TEXT)
                );

                return true;
            } else {
                println!(
                    "{}",
                    format!(
                        "Mossa non valida da ({}, {}) a ({}, {})",
                        s_c, s_r, e_c, e_r
                    )
                    .with(ERROR_TEXT)
                );
                return false; // Mossa non valida
            }
        } else {
            println!(
                "{}",
                "Errore nell'input, uno dei valori è None.".with(ERROR_TEXT)
            );
            return false;
        }
    } else {
        println!(
            "{}",
            "Formato di input errato. Usa ad es. 'e2-e4'".with(ERROR_TEXT)
        );
        return false;
    }
}

fn end_game_screen(result: &str) -> bool {
    clear_console();
    println!("\n\n\n\n\n");
    let (x, _) = get_terminal_size().expect("Errore nel ottenere la dimensione del terminale");
    spacex_off(x, None);

    // Mostra il risultato della partita
    let message = match result {
        "Checkmate" => "Scacco Matto!".with(SUCCESS_TEXT).to_string(),
        "Stalemate" => "Stallo!".with(SUCCESS_TEXT).to_string(),
        _ => "".to_string(),
    };

    println!(
        "{}\nVuoi iniziare una nuova partita? (s per sì / qualsiasi altro tasto per uscire)",
        message
    );

    spacex_off(x, None);
    let input = get_input(": ");

    // Se l'utente sceglie 's', riavvia la partita
    input.trim().to_lowercase() == "s"
}

fn col_to_number(c: char) -> Option<u32> {
    if ('a'..='i').contains(&c) {
        Some((c as u32) - ('a' as u32))
    } else {
        None
    }
}

fn row_to_number(c: char) -> Option<u32> {
    if ('1'..='9').contains(&c) {
        Some((c as u32) - ('1' as u32))
    } else {
        None
    }
}

fn is_valid_turn(piece_id: i32, gm: i32) -> bool {
    (piece_id % 2 == 0 && gm % 2 == 0) || (piece_id % 2 == 1 && gm % 2 == 1)
}

fn check_piece(
    matrix: &mut [[i32; 8]; 8],
    id: i32,
    mc: i32,
    mr: i32,
    s_r: i32,
    s_c: i32,
    enemy: i32,
) -> bool {
    match id {
        10 | 11 => {
            // Pawn
            let direction = if id == 10 { -1 } else { 1 }; // I pedoni bianchi si muovono verso l'alto (-1), i pedoni neri verso il basso (+1)
            let start_row = if id == 10 { 6 } else { 1 }; // Riga di partenza per i pedoni bianchi (6) e neri (1)

            if enemy == 0 {
                // Mossa normale
                if mc == 0 && mr == direction {
                    return true;
                } else if mc == 0 && mr == 2 * direction && (start_row == s_r) {
                    // Controllo se la casella direttamente davanti al pedone è vuota
                    let middle_square_row = s_r + direction; // Riga direttamente davanti al pedone
                    if middle_square_row >= 0
                        && middle_square_row < 8
                        && matrix[middle_square_row as usize][s_c as usize] == 0
                    {
                        return true;
                    }
                }
            } else {
                // Mossa di cattura
                if mc.abs() == 1 && mr == direction {
                    return true;
                }
            }
        }

        20 | 21 => {
            // Bishop (Alfiere)
            if mc.abs() == mr.abs() {
                // Movimento diagonale
                let step_x = if mc > 0 { 1 } else { -1 };
                let step_y = if mr > 0 { 1 } else { -1 };
                let mut x = s_c + step_x;
                let mut y = s_r + step_y;

                // Controlla tutte le caselle lungo la diagonale, ma non include la destinazione
                while x != s_c + mc && y != s_r + mr {
                    if matrix[y as usize][x as usize] != 0 {
                        return false; // Ostacolo sulla diagonale
                    }
                    // Metti 99 nella casella controllata per tracciare le caselle
                    //matrix[y as usize][x as usize] = 99;
                    x += step_x;
                    y += step_y;
                }
                return true; // Nessun ostacolo trovato
            }
        }
        30 | 31 => {
            // Knight (Cavallo)
            if (mc.abs() == 2 && mr.abs() == 1) || (mc.abs() == 1 && mr.abs() == 2) {
                return true; // Il cavallo non ha bisogno di verificare ostacoli
            }
        }
        40 | 41 => {
            // Rook (Torre)
            if mc == 0 || mr == 0 {
                // Movimento in linea retta (orizzontale o verticale)
                let (start, end) = if mc == 0 {
                    // Movimento verticale
                    (s_r.min(s_r + mr), s_r.max(s_r + mr))
                } else {
                    // Movimento orizzontale
                    (s_c.min(s_c + mc), s_c.max(s_c + mc))
                };

                for i in (start + 1)..end {
                    if mc == 0 && matrix[i as usize][s_c as usize] != 0 {
                        return false; // Ostacolo sulla colonna
                    }
                    if mr == 0 && matrix[s_r as usize][i as usize] != 0 {
                        return false; // Ostacolo sulla riga
                    }
                }
                return true;
            }
        }
        50 | 51 => {
            // Queen (Regina)
            if mc == 0 || mr == 0 || mc.abs() == mr.abs() {
                // Movimento come torre (orizzontale o verticale) o come alfiere (diagonale)
                if mc == 0 || mr == 0 {
                    // Movimento orizzontale/verticale (come torre)
                    let (start, end) = if mc == 0 {
                        // Movimento verticale
                        (s_r.min(s_r + mr), s_r.max(s_r + mr))
                    } else {
                        // Movimento orizzontale
                        (s_c.min(s_c + mc), s_c.max(s_c + mc))
                    };

                    for i in (start + 1)..end {
                        if mc == 0 && matrix[i as usize][s_c as usize] != 0 {
                            return false; // Ostacolo sulla colonna
                        }
                        if mr == 0 && matrix[s_r as usize][i as usize] != 0 {
                            return false; // Ostacolo sulla riga
                        }
                    }
                    return true;
                } else {
                    // Movimento diagonale (come alfiere)
                    let step_x = if mc > 0 { 1 } else { -1 };
                    let step_y = if mr > 0 { 1 } else { -1 };
                    let mut x = s_c + step_x;
                    let mut y = s_r + step_y;

                    // Controlla tutte le caselle lungo la diagonale
                    while x != s_c + mc && y != s_r + mr {
                        if matrix[y as usize][x as usize] != 0 {
                            return false; // Ostacolo sulla diagonale
                        }
                        x += step_x;
                        y += step_y;
                    }
                    return true;
                }
            }
        }
        60 | 61 => {
            // King (Re)
            if mc.abs() <= 1 && mr.abs() <= 1 {
                return true;
            }
            if mr == 0 && mc.abs() == 2 {
                return true; 
            }
        }
        _ => {
            println!("Unknown piece with id: {}", id);
            return false;
        }
    }
    println!("Invalid move for id: {} with mc: {} and mr: {}", id, mc, mr);
    false
}

fn check_king(
    s_r: i32,
    s_c: i32,
    e_r: i32,
    e_c: i32,
    gm_round: i32,
    matrix: &[[i32; 8]; 8],
) -> bool {
    let mut cloned_matrix = matrix.clone();

    // Aggiorna la matrice clonata con la mossa simulata
    if s_r != -1 || s_c != -1 || e_r != -1 || e_c != -1 {
        cloned_matrix[s_r as usize][s_c as usize] = 0;
        cloned_matrix[e_r as usize][e_c as usize] = matrix[s_r as usize][s_c as usize];
    }

    let direction: i32 = gm_round % 2; // 0 per bianco, 1 per nero
    let target: i32 = 60 + direction; // Re bianco = 60, re nero = 61

    // Trova la posizione del re nella matrice
    let mut king_row: i32 = -1;
    let mut king_col: i32 = -1;

    for x in 0..8 {
        for y in 0..8 {
            if cloned_matrix[x][y] == target {
                king_row = x as i32;
                king_col = y as i32;
                break;
            }
        }
    }

    //println!("Re si trova a: ({}, {})", king_row, king_col);

    // Controlla se ci sono minacce da parte di pedoni avversari
    // Verifica che il pedone appartenga all'avversario e non sia del proprio colore
    if king_row + direction >= 0 && king_row + direction < 8 {
        if (cloned_matrix[(king_row + 1 * direction) as usize][king_col as usize]
            == (11 - direction))
            || (king_col - 1 >= 0
                && cloned_matrix[(king_row + 1 * direction) as usize][(king_col - 1) as usize]
                    == (11 - direction))
            || (king_col + 1 < 8
                && cloned_matrix[(king_row + 1 * direction) as usize][(king_col + 1) as usize]
                    == (11 - direction))
        {
            //println!("Re sotto scacco da un pedone avversario!");
            return false;
        }
    }

    // Controlla se ci sono minacce da parte di cavalli avversari
    let knight_moves: [(i32, i32); 8] = [
        (2, 1),
        (2, -1),
        (-2, 1),
        (-2, -1),
        (1, 2),
        (1, -2),
        (-1, 2),
        (-1, -2),
    ];

    for (dx, dy) in knight_moves.iter() {
        let nx = king_row + dx;
        let ny = king_col + dy;
        if nx >= 0 && nx < 8 && ny >= 0 && ny < 8 {
            if cloned_matrix[nx as usize][ny as usize] == (31 - direction) {
                //println!("Re sotto scacco da un cavallo avversario!");
                return false;
            }
        }
    }

    // controllo alfieri nemici
    let bishop_moves: [(i32, i32); 4] = [
        (1, 1),   // basso-destra
        (1, -1),  // basso-sinistra
        (-1, 1),  // alto-destra
        (-1, -1), // alto-sinistra
    ];

    for (dx, dy) in bishop_moves.iter() {
        let mut nx = king_row + dx;
        let mut ny = king_col + dy;

        while nx >= 0 && nx < 8 && ny >= 0 && ny < 8 {
            let piece = cloned_matrix[nx as usize][ny as usize];

            if piece != 0 {
                if piece == (21 - direction) || piece == (51 - direction) {
                    return false;
                }
                break; // Se troviamo un pezzo che non è una minaccia, interrompiamo il controllo in questa direzione
            }

            // Continua a muoverti nella diagonale
            nx += dx;
            ny += dy;
        }
    }

    // Controlla se ci sono minacce da parte di torri avversarie
    let rook_moves: [(i32, i32); 4] = [
        (1, 0),  // giù
        (-1, 0), // su
        (0, 1),  // destra
        (0, -1), // sinistra
    ];

    for (dx, dy) in rook_moves.iter() {
        let mut nx = king_row + dx;
        let mut ny = king_col + dy;

        while nx >= 0 && nx < 8 && ny >= 0 && ny < 8 {
            let piece = cloned_matrix[nx as usize][ny as usize];

            if piece != 0 {
                if piece == (41 - direction) || piece == (51 - direction) {
                    // 41 - direction: Torre avversaria
                    // 51 - direction: Regina avversaria
                    return false; // Il re è sotto scacco
                }
                break; // C'è un altro pezzo che blocca la linea di vista
            }

            // Continua a muoverti nella stessa direzione
            nx += dx;
            ny += dy;
        }
    }

    //println!("Re non è sotto scacco");
    true
}

fn king_trick(matrix: &mut [[i32; 8]; 8], gm_round: &mut i32, long_castling: bool) -> bool {
    let is_white_turn = *gm_round % 2 == 0;
    let (king_row, king_col, rook_col, king_target_col, rook_target_col) = if is_white_turn {
        if long_castling {
            (7, 4, 0, 2, 3) // Arrocco lungo per il bianco
        } else {
            (7, 4, 7, 6, 5) // Arrocco corto per il bianco
        }
    } else {
        if long_castling {
            (0, 4, 0, 2, 3) // Arrocco lungo per il nero
        } else {
            (0, 4, 7, 6, 5) // Arrocco corto per il nero
        }
    };

    // Verifica se il re e la torre non hanno mosso e nessun pezzo è tra di loro
    let king = matrix[king_row][king_col];
    let rook = matrix[king_row][rook_col];

    if !((king == 60 && is_white_turn) || (king == 61 && !is_white_turn)) {
        println!("Errore: Il re ha già mosso o non è il turno corretto.");
        return false;
    }

    if !((rook == 40 && is_white_turn) || (rook == 41 && !is_white_turn)) {
        println!("Errore: La torre ha già mosso o non è il turno corretto.");
        return false;
    }

    // Controllo che non ci siano pezzi tra il re e la torre
    let (start_col, end_col) = if long_castling {
        (1, king_col - 1)
    } else {
        (king_col + 1, rook_col - 1)
    };

    for col in start_col..=end_col {
        if matrix[king_row][col] != 0 {
            println!("Errore: Ci sono pezzi tra il re e la torre.");
            return false;
        }
    }

    // Controllo se il re è sotto scacco o attraversa caselle minacciate
    if !check_king(
        king_row as i32,
        king_col as i32,
        king_row as i32,
        king_target_col as i32,
        *gm_round,
        matrix,
    ) {
        println!("Errore: Il re è sotto scacco o attraversa caselle minacciate.");
        return false;
    }

    // Esegui l'arrocco: sposta il re e la torre
    matrix[king_row][king_target_col] = king; // Sposta il re
    matrix[king_row][rook_target_col] = rook; // Sposta la torre
    matrix[king_row][king_col] = 0; // Libera la posizione originale del re
    matrix[king_row][rook_col] = 0; // Libera la posizione originale della torre

    *gm_round += 1;

    println!(
        "{} arrocco eseguito per il {}",
        if long_castling { "Lungo" } else { "Corto" },
        if is_white_turn { "bianco" } else { "nero" }
    );
    true
}

fn touchdown(e_c: i32, e_s: i32, matrix: &mut [[i32; 8]; 8], gm: i32) {
    let input = get_input("TouchDown (2-3-4-5) : ");
    //let clear = input.trim().chars().next().and_then(row_to_number);

    if let Some(first_char) = input.trim().chars().next() {
        // Prova a convertire il carattere a numero
        if let Some(mut clear) = row_to_number(first_char) {
            clear += 1; // Incrementa il valore di clear

            println!("Clear value: {:?}", clear);
            let newboss = (clear * 10) + (gm as u32 % 2);

            matrix[e_s as usize][e_c as usize] = newboss as i32;
        } else {
            println!("Input non valido: il carattere non è un numero valido");
        }
    } else {
        println!("Input non valido: stringa vuota");
    }

    // usando gm per cambiare il pezzo
}

// Funzione per verificare se il pezzo mangiato appartiene alla propria squadra
fn check_eat(matrix: &[[i32; 8]; 8], s_r: i32, s_c: i32, e_r: i32, e_c: i32) -> bool {
    let start_piece = matrix[s_r as usize][s_c as usize];
    let end_piece = matrix[e_r as usize][e_c as usize];
    if end_piece != 0 && (start_piece % 2 == end_piece % 2) {
        return false; // Non puoi mangiare un tuo pezzo
    }
    true
}

// Funzione per catturare un pezzo mangiato e inserirlo nell'array corrispondente
fn capture_piece(pieces: &mut [i32; 16], piece: i32) {
    if piece == 0 {
        return;
    }
    let mut index = 0;
    while index < pieces.len() && pieces[index] != 0 {
        if pieces[index] > piece {
            break;
        }
        index += 1;
    }
    for i in (index..pieces.len() - 1).rev() {
        pieces[i + 1] = pieces[i];
    }
    pieces[index] = piece;
}

// Funzione per stampare i pezzi catturati
fn print_captured_pieces(black_pieces: &[i32; 16], white_pieces: &[i32; 16], screenx: u16) {
    let piece_unicode = |num: i32| -> &str {
        match num {
            1 => "\u{f0859} ", // Pedone
            2 => "\u{f085c} ", // Alfiere
            3 => "\u{f0858} ", // Cavallo
            4 => "\u{f085b} ", // Torre
            5 => "\u{f085a} ", // Regina
            6 => "\u{f0857} ", // Re
            _ => "xx",         // Errore
        }
    };

    println!(" ");
    spacex_off(screenx, Some(2));

    // Stampa pezzi neri
    for (i, piece) in black_pieces.iter().enumerate() {
        if i > 0 && i % 8 == 0 {
            println!();
            spacex_off(screenx, Some(2));
        }
        if *piece != 0 {
            // Usa il testo nero (TEXT2) e il background nero (BACKGROUND1) per i pezzi neri
            print!(
                "{}",
                piece_unicode((piece / 10) as i32)
                    .with(TEXT2)
                    .on(BACKGROUND1)
            );
        } else {
            // Stampa spazio vuoto con il background nero (BACKGROUND1)
            print!("{}", "  ".on(BACKGROUND1));
        }
    }
    println!();

    spacex_off(screenx, Some(2));

    // Stampa pezzi bianchi
    for (i, piece) in white_pieces.iter().enumerate() {
        if i > 0 && i % 8 == 0 {
            println!();
            spacex_off(screenx, Some(2));
        }
        if *piece != 0 {
            // Usa il testo bianco (TEXT1) e il background bianco (BACKGROUND2) per i pezzi bianchi
            print!(
                "{}",
                piece_unicode((piece / 10) as i32)
                    .with(TEXT1)
                    .on(BACKGROUND2)
            );
        } else {
            // Stampa spazio vuoto con il background bianco (BACKGROUND2)
            print!("{}", "  ".on(BACKGROUND2));
        }
    }
    println!();
}

fn fastturn(gm: i32, screenx: u16) {
    spacex_off(screenx, None);
    let colore = if gm % 2 == 0 {
        "white".with(SUCCESS_TEXT)
    } else {
        "black".with(ERROR_TEXT)
    };
    println!(" Turno {} - {}", gm + 1, colore);
}

fn check_stalemate_checkmate(matrix: &[[i32; 8]; 8], gm_round: i32) -> String {
    let player_color = gm_round % 2; // 0 per bianco, 1 per nero
    let opponent_color = 1 - player_color; // Colore dell'avversario
    let mut king_pos = (-1, -1);

    // Trova la posizione del re dell'avversario
    for i in 0..8 {
        for j in 0..8 {
            if matrix[i][j] == 60 + opponent_color {
                king_pos = (i as i32, j as i32);
                break;
            }
        }
    }

    let is_in_check = !check_king(
        king_pos.0, king_pos.1, king_pos.0, king_pos.1, gm_round, matrix,
    );
    let mut has_legal_move = false;

    // Controlla se l'avversario ha mosse legali
    'outer: for i in 0..8 {
        for j in 0..8 {
            let piece = matrix[i][j];
            if piece != 0 && (piece % 2 == opponent_color) {
                for x in 0..8 {
                    for y in 0..8 {
                        if check_piece(
                            &mut matrix.clone(),
                            piece,
                            y as i32 - j as i32,
                            x as i32 - i as i32,
                            i as i32,
                            j as i32,
                            matrix[x][y],
                        ) && check_king(
                            i as i32,
                            j as i32,
                            x as i32,
                            y as i32,
                            gm_round + 1,
                            matrix,
                        ) {
                            has_legal_move = true;
                            break 'outer;
                        }
                    }
                }
            }
        }
    }

    if is_in_check && !has_legal_move {
        "Checkmate".to_string()
    } else if !is_in_check && !has_legal_move {
        "Stalemate".to_string()
    } else if is_in_check {
        "Check".to_string()
    } else {
        "Ongoing".to_string()
    }
}

// Converte la matrice della scacchiera in notazione FEN
fn matrix_to_fen(matrix: &[[i32; 8]; 8]) -> String {
    let mut fen = String::new();
    for row in matrix.iter() {
        let mut empty_count = 0;
        for &piece in row.iter() {
            let piece_char = match piece {
                10 => 'P',
                11 => 'p',
                20 => 'B',
                21 => 'b',
                30 => 'N',
                31 => 'n',
                40 => 'R',
                41 => 'r',
                50 => 'Q',
                51 => 'q',
                60 => 'K',
                61 => 'k',
                0 => {
                    empty_count += 1;
                    continue;
                }
                _ => continue,
            };
            if empty_count > 0 {
                fen.push_str(&empty_count.to_string());
                empty_count = 0;
            }
            fen.push(piece_char);
        }
        if empty_count > 0 {
            fen.push_str(&empty_count.to_string());
        }
        fen.push('/');
    }
    fen.pop(); // Rimuove l'ultimo '/'
    fen
}

// Converte una stringa FEN nella matrice della scacchiera
fn fen_to_matrix(fen: &str) -> [[i32; 8]; 8] {
    let mut matrix = [[0; 8]; 8];
    let rows: Vec<&str> = fen.split('/').collect();

    for (i, row) in rows.iter().enumerate() {
        let mut j = 0;
        for c in row.chars() {
            if c.is_digit(10) {
                j += c.to_digit(10).unwrap() as usize;
            } else {
                let piece = match c {
                    'P' => 10,
                    'p' => 11,
                    'B' => 20,
                    'b' => 21,
                    'N' => 30,
                    'n' => 31,
                    'R' => 40,
                    'r' => 41,
                    'Q' => 50,
                    'q' => 51,
                    'K' => 60,
                    'k' => 61,
                    _ => 0,
                };
                matrix[i][j] = piece;
                j += 1;
            }
        }
    }
    matrix
}

fn get_best_move(fen: &str, player_color: i32, gm_round: i32, depth: i32) -> String {
    let mut engine = Command::new("stockfish")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Impossibile avviare Stockfish");

    let stdin = engine
        .stdin
        .as_mut()
        .expect("Errore nell'accedere allo stdin");
    let stdout = engine
        .stdout
        .as_mut()
        .expect("Errore nell'accedere allo stdout");
    let mut reader = BufReader::new(stdout);

    writeln!(stdin, "uci").expect("Errore nell'invio del comando UCI");
    writeln!(stdin, "isready").expect("Errore nell'invio del comando isready");

    // Aspetta che il motore sia pronto
    for line in reader.by_ref().lines() {
        let line = line.unwrap();
        if line.contains("readyok") {
            break;
        }
    }

    let color = if player_color == 0 { "w" } else { "b" };
    let fen_with_turn = format!("{} {} KQkq - 0 {}", fen, color, (gm_round / 2) + 1);

    // Imposta la posizione e richiedi la miglior mossa con il depth specificato
    writeln!(stdin, "position fen {}", fen_with_turn).expect("Errore nell'impostare la posizione");
    writeln!(stdin, "go depth {}", depth).expect("Errore nell'invio del comando di ricerca");

    for line in reader.lines() {
        let line = line.unwrap();
        if line.starts_with("bestmove") {
            let best_move = line.split_whitespace().nth(1).unwrap().to_string();
            return best_move;
        }
    }

    String::from("nessuna mossa trovata")
}

fn get_best_move_with_evaluation(fen: &str, player_color: i32, gm_round: i32) -> (String, String) {
    let mut engine = Command::new("stockfish")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Impossibile avviare Stockfish");

    let stdin = engine
        .stdin
        .as_mut()
        .expect("Errore nell'accedere allo stdin");
    let stdout = engine
        .stdout
        .as_mut()
        .expect("Errore nell'accedere allo stdout");
    let mut reader = BufReader::new(stdout);

    writeln!(stdin, "uci").expect("Errore nell'invio del comando UCI");
    writeln!(stdin, "isready").expect("Errore nell'invio del comando isready");

    // Legge l'output fino alla conferma di 'readyok'
    for line in reader.by_ref().lines() {
        let line = line.unwrap();
        if line.contains("readyok") {
            break;
        }
    }

    let color = if player_color == 0 { "w" } else { "b" };
    let fen_with_turn = format!("{} {} KQkq - 0 {}", fen, color, (gm_round / 2) + 1);

    writeln!(stdin, "position fen {}", fen_with_turn).expect("Errore nell'impostare la posizione");
    writeln!(stdin, "go depth 10").expect("Errore nell'invio del comando di ricerca");

    let mut best_move = String::new();
    let mut evaluation = String::from("Nessuna valutazione trovata");

    for line in reader.lines() {
        let line = line.unwrap();
        if line.starts_with("bestmove") {
            best_move = line.split_whitespace().nth(1).unwrap().to_string();
            break;
        } else if line.contains("info score") {
            if line.contains("cp") {
                let score = line
                    .split_whitespace()
                    .find(|&s| s.parse::<i32>().is_ok())
                    .unwrap();
                evaluation = format!("Valutazione in centipawn: {}", score);
            } else if line.contains("mate") {
                let mate_in = line
                    .split_whitespace()
                    .find(|&s| s.parse::<i32>().is_ok())
                    .unwrap();
                evaluation = format!("Matto in {} mosse", mate_in);
            }
        }
    }

    (best_move, evaluation)
}

fn display_game_result(
    result: &str,
    winner: &str,
    gm_round: i32,
    captured_black: &[i32; 16],
    captured_white: &[i32; 16],
) {
    clear_console();
    println!("\n\n\n\n\n");

    let (x, _) = get_terminal_size().expect("Errore nel ottenere la dimensione del terminale");
    spacex_off(x, None);

    // Mostra il messaggio di fine partita
    let message = match result {
        "Checkmate" => format!("Scacco Matto! {} ha vinto.", winner)
            .with(SUCCESS_TEXT)
            .to_string(),
        "Stalemate" => "Stallo! Il gioco è finito in parità."
            .with(SUCCESS_TEXT)
            .to_string(),
        "Resign" => format!(
            "{} si è arreso! {} ha vinto.",
            winner,
            if winner == "bianco" { "nero" } else { "bianco" }
        )
        .with(SUCCESS_TEXT)
        .to_string(),
        _ => "".to_string(),
    };

    println!("{}", message);

    // Mostra un riepilogo del gioco
    println!("\nRiepilogo della partita:");
    println!("Turni giocati: {}", gm_round);

    println!("Pezzi neri catturati: ");
    for piece in captured_black.iter().filter(|&&p| p != 0) {
        print!("{} ", piece);
    }

    println!("\nPezzi bianchi catturati: ");
    for piece in captured_white.iter().filter(|&&p| p != 0) {
        print!("{} ", piece);
    }

    println!("\nVuoi iniziare una nuova partita? (s per sì / qualsiasi altro tasto per uscire)");

    spacex_off(x, None);
    let input = get_input(": ");

    // Se l'utente sceglie 's', riavvia la partita
    if input.trim().to_lowercase() == "s" {
        clear_console();
    } else {
        println!("Grazie per aver giocato!");
        std::process::exit(0);
    }
}

fn end_game_prompt() -> bool {
    clear_console();
    println!("\n\n\n\n\n");

    let (x, _) = get_terminal_size().expect("Errore nel ottenere la dimensione del terminale");
    spacex_off(x, None);

    // Mostra il messaggio di fine partita
    println!(
        "{}",
        "La partita è finita! Vuoi iniziare una nuova partita? (s per sì / qualsiasi altro tasto per uscire)"
            .with(SUCCESS_TEXT)
    );

    spacex_off(x, None);
    let input = get_input(": ");

    // Se l'utente sceglie 's', ritorna vero per riavviare la partita
    input.trim().to_lowercase() == "s"
}

fn main() {
    loop {
        let mut scacchiera: [[i32; 8]; 8] = [[0; 8]; 8];
        let mut gm_round: i32 = 0;

        let mut en_passant_row: i32 = -1;
        let mut en_passant_col: i32 = -1;

        let mut show_piece: bool = false;
        let mut black_piece: [i32; 16] = [0; 16];
        let mut white_piece: [i32; 16] = [0; 16];

        let mut enemy_active: bool = false;
        let mut enemy_color: String = "black".to_string(); // Il colore del nemico è inizialmente nero

        let mut depth: i32 = 5; // Profondità iniziale
        let mut aggressiveness: i32 = 1; // Livello di "cattiveria" del nemico

        let mut show_fen: bool = false; // Variabile per controllare se mostrare il FEN
        let mut show_suggested_move: bool = false; // Variabile per controllare se mostrare la mossa consigliata

        reset_word(&mut scacchiera);

        loop {
            clear_console();

            let (x, y) =
                get_terminal_size().expect("Errore nel ottenere la dimensione del terminale");
            spacey(y);
            fastturn(gm_round, x + 1);
            println!(" ");

            fx_stamp_word(&mut scacchiera, &mut gm_round, x);
            if show_piece {
                print_captured_pieces(&black_piece, &white_piece, x);
            }

            // Mostra il FEN solo se show_fen è true
            if show_fen {
                let fen = matrix_to_fen(&scacchiera);
                spacex_off(x, Some(-4));
                print!("{}", fen);
            }

            // Controlla se il gioco è finito (checkmate, stalemate o nessuna mossa legale)
            let best_move =
                get_best_move(&matrix_to_fen(&scacchiera), gm_round % 2, gm_round, depth);

            //println!("the best of the best is {}", best_move);

            if best_move == "(none)" {
                let game_state = check_stalemate_checkmate(&scacchiera, gm_round);
                let winner = if gm_round % 2 == 1 { "bianco" } else { "nero" };

                if game_state == "Checkmate" {
                    display_game_result("Checkmate", winner, gm_round, &black_piece, &white_piece);
                } else if game_state == "Stalemate" {
                    display_game_result(
                        "Stalemate",
                        "nessuno",
                        gm_round,
                        &black_piece,
                        &white_piece,
                    );
                } else {
                    println!("Partita terminata, nessuna mossa valida disponibile.");
                }

                break; // Esce dal ciclo di gioco per andare al prompt di fine partita
            }

            // Mostra la mossa suggerita solo se show_suggested_move è true
            if show_suggested_move {
                spacex_off(x, None);
                print!("Mossa suggerita da Stockfish: {}", best_move);
            }

            println!();

            // Se il nemico è attivo e tocca al nemico, facciamo muovere il nemico automaticamente
            if enemy_active
                && ((enemy_color == "white" && gm_round % 2 == 0)
                    || (enemy_color == "black" && gm_round % 2 == 1))
            {
                println!("Il nemico sta facendo la sua mossa...");
                let enemy_move =
                    get_best_move(&matrix_to_fen(&scacchiera), gm_round % 2, gm_round, depth);

                if enemy_move == "none" {
                    let winner = if gm_round % 2 == 1 { "bianco" } else { "nero" };
                    display_game_result("Checkmate", winner, gm_round, &black_piece, &white_piece);
                    break;
                } else {
                    println!("Mossa del nemico: {}", enemy_move);
                    pz_move(
                        &enemy_move,
                        &mut gm_round,
                        &mut scacchiera,
                        &mut black_piece,
                        &mut white_piece,
                        &mut en_passant_row,
                        &mut en_passant_col,
                    );
                    continue; // Passa direttamente al turno successivo dopo la mossa del nemico
                }
            }

            // Spazi per l'input dell'utente
            spacex_off(x, None);
            let input = get_input(": ");

            // Gestione dei vari comandi utente
            if input.trim() == "enemy on" {
                enemy_active = true;
                println!("Nemico attivato.");
            } else if input.trim() == "help" {
                println!("\n=== COMANDI DISPONIBILI ===");
                println!("  help                     - Mostra questo elenco di comandi.");
                println!("  enemy on                 - Attiva il nemico controllato dall'AI.");
                println!("  enemy off                - Disattiva il nemico controllato dall'AI.");
                println!("  enemy set [white|black]  - Imposta il nemico su bianco o nero.");
                println!("  set depth [n]            - Imposta la profondità della ricerca AI.");
                println!("  set aggressiveness [n]   - Imposta il livello di aggressività dell'AI.");
                println!("  0-0                      - Arrocco corto.");
                println!("  0-0-0                    - Arrocco lungo.");
                println!("  show piece               - Mostra/nasconde i pezzi catturati.");
                println!("  show fen                 - Mostra/nasconde la posizione FEN.");
                println!("  show suggested move      - Mostra/nasconde la mossa suggerita.");
                println!("  import fen [FEN]         - Importa una posizione FEN.");
                println!("  exit                     - Esci dal gioco.");
                println!("  [e2-e4] o [e2e4]         - Esegui una mossa.");
                println!("===========================\n");
                println!("Premi Invio per tornare alla partita...");
                let mut dummy = String::new();
                io::stdin().read_line(&mut dummy).expect("Errore");
            } else if input.trim() == "enemy off" {
                enemy_active = false;
                println!("Nemico disattivato.");
            } else if input.trim().starts_with("enemy set ") {
                let args: Vec<String> = input
                    .trim()
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                if args.len() == 3 {
                    if args[2] == "white" || args[2] == "black" {
                        enemy_color = args[2].clone();
                        println!("Il nemico ora gioca come {}", enemy_color);
                    } else {
                        println!("Colore non valido. Usa 'white' o 'black'.");
                    }
                }
            } else if input.trim().starts_with("set depth ") {
                if let Ok(new_depth) = input
                    .trim()
                    .split_whitespace()
                    .nth(2)
                    .unwrap()
                    .parse::<i32>()
                {
                    depth = new_depth;
                    println!("Profondità impostata a: {}", depth);
                } else {
                    println!("Errore: Profondità non valida");
                }
            } else if input.trim().starts_with("set aggressiveness ") {
                if let Ok(new_aggressiveness) = input
                    .trim()
                    .split_whitespace()
                    .nth(2)
                    .unwrap()
                    .parse::<i32>()
                {
                    aggressiveness = new_aggressiveness;
                    println!("Livello di cattiveria impostato a: {}", aggressiveness);
                } else {
                    println!("Errore: Livello di cattiveria non valido");
                }
            } else if input.trim() == "exit" {
                break;
            } else if input.trim() == "0-0-0" {
                king_trick(&mut scacchiera, &mut gm_round, true);
            } else if input.trim() == "0-0" {
                king_trick(&mut scacchiera, &mut gm_round, false);
            } else if input.trim() == "show piece" {
                show_piece = !show_piece;
            } else if input.trim() == "show fen" {
                show_fen = !show_fen;
                println!("Mostra FEN impostato su: {}", show_fen);
            } else if input.trim() == "show suggested move" {
                show_suggested_move = !show_suggested_move;
                println!(
                    "Mostra mossa consigliata impostato su: {}",
                    show_suggested_move
                );
            } else if input.trim().starts_with("import fen ") {
                let fen_input = input.trim().replace("import fen ", "");
                scacchiera = fen_to_matrix(&fen_input);
                println!("Posizione FEN importata con successo.");
            } else if pz_move(
                input.trim(),
                &mut gm_round,
                &mut scacchiera,
                &mut black_piece,
                &mut white_piece,
                &mut en_passant_row,
                &mut en_passant_col,
            ) {
                let game_state = check_stalemate_checkmate(&scacchiera, gm_round);

                match game_state.as_str() {
                    "Checkmate" => {
                        let winner = if gm_round % 2 == 1 { "bianco" } else { "nero" };
                        display_game_result(
                            "Checkmate",
                            winner,
                            gm_round,
                            &black_piece,
                            &white_piece,
                        );
                        break;
                    }
                    "Stalemate" => {
                        display_game_result(
                            "Stalemate",
                            "nessuno",
                            gm_round,
                            &black_piece,
                            &white_piece,
                        );
                        break;
                    }
                    "Check" => {
                        println!("Scacco al re!");
                    }
                    _ => {}
                }
            } else {
                println!("Mossa non valida, riprova.");
            }
        }

        // Chiede all'utente se vuole giocare di nuovo o uscire
        if !end_game_prompt() {
            break;
        }
    }
}
