//! Översätter svensk dragnotation till standard-SAN.
//!
//! Svenska pjäsbokstäver: K (kung), D (dam), T (torn), L (löpare), S (springare).
//! Drag kan även anges med pjäsens namn i klartext, som "springare f3".

use std::iter::Peekable;
use std::str::Chars;

/// Standard-SAN-bokstav för en svensk pjäsbokstav.
const fn piece_letter(swedish: char) -> Option<char> {
    match swedish {
        'K' => Some('K'),
        'D' => Some('Q'),
        'T' => Some('R'),
        'L' => Some('B'),
        'S' => Some('N'),
        _ => None,
    }
}

/// Standard-SAN-prefix för ett svenskt pjäsnamn. Bonde har inget prefix.
fn piece_word(word: &str) -> Option<&'static str> {
    match word {
        "kung" => Some("K"),
        "dam" => Some("Q"),
        "torn" => Some("R"),
        "löpare" => Some("B"),
        "springare" => Some("N"),
        "bonde" => Some(""),
        _ => None,
    }
}

/// Är `c` en kolumn a–h?
const fn is_file(c: char) -> bool {
    matches!(c, 'a'..='h')
}

/// Är `c` en rad 1–8?
const fn is_rank(c: char) -> bool {
    matches!(c, '1'..='8')
}

/// Plockar en sammanhängande följd av kolumn- och radtecken.
fn take_squares(chars: &mut Peekable<Chars<'_>>) -> Vec<char> {
    let mut taken = Vec::new();
    while let Some(c) = chars.next_if(|c| is_file(*c) || is_rank(*c)) {
        taken.push(c);
    }
    taken
}

/// Är `s` exakt en ruta, som `e4`?
fn is_square(s: &[char]) -> bool {
    matches!(s, [file, rank] if is_file(*file) && is_rank(*rank))
}

/// Är `s` en giltig särskiljare: tom, en kolumn, en rad eller en hel ruta?
fn is_disambiguation(s: &[char]) -> bool {
    match s {
        [] | [_] => true,
        [file, rank] => is_file(*file) && is_rank(*rank),
        _ => false,
    }
}

/// Validerar och översätter delen efter pjäsprefixet:
/// `[särskiljare][x]<ruta>[=pjäs][+|#]`.
// ponytail: handskriven parser i stället för regex, så domänet slipper beroenden.
fn translate_tail(tail: &str) -> Option<String> {
    let mut chars = tail.chars().peekable();
    let mut disambiguation = take_squares(&mut chars);
    let capture = chars.next_if_eq(&'x').is_some();
    let destination = if capture {
        take_squares(&mut chars)
    } else {
        disambiguation.split_off(disambiguation.len().checked_sub(2)?)
    };
    if !is_square(&destination) || !is_disambiguation(&disambiguation) {
        return None;
    }

    let mut out: String = disambiguation.into_iter().collect();
    if capture {
        out.push('x');
    }
    out.extend(destination);

    if chars.next_if_eq(&'=').is_some() {
        let promoted = chars.next().and_then(piece_letter)?;
        if promoted == 'K' {
            return None;
        }
        out.push('=');
        out.push(promoted);
    }

    if let Some(suffix) = chars.next_if(|c| matches!(c, '+' | '#')) {
        out.push(suffix);
    }
    chars.next().is_none().then_some(out)
}

/// Översätter standard-SAN till svenska pjäsbokstäver, som `Nf3` till `Sf3`.
#[must_use]
pub fn to_swedish(san: &str) -> String {
    san.chars()
        .map(|c| match c {
            'N' => 'S',
            'B' => 'L',
            'R' => 'T',
            'Q' => 'D',
            other => other,
        })
        .collect()
}

/// Översätter ett drag i svensk notation till standard-SAN.
///
/// Tar emot "Sf3", "springare f3", "e4", "Dxe5+", "0-0" och "e8=D".
/// Returnerar `None` om draget inte känns igen.
#[must_use]
pub fn to_san(input: &str) -> Option<String> {
    let text = input.split_whitespace().collect::<Vec<_>>().join(" ");

    match text.as_str() {
        "0-0" | "O-O" => return Some("O-O".into()),
        "0-0-0" | "O-O-O" => return Some("O-O-O".into()),
        _ => {}
    }

    if let Some((word, rest)) = text.split_once(' ') {
        let prefix = piece_word(&word.to_lowercase())?;
        return translate_tail(rest).map(|tail| format!("{prefix}{tail}"));
    }

    let mut chars = text.chars();
    let first = chars.next()?;
    piece_letter(first).map_or_else(
        || translate_tail(&text),
        |letter| translate_tail(chars.as_str()).map(|tail| format!("{letter}{tail}")),
    )
}

#[cfg(test)]
#[allow(
    clippy::missing_docs_in_private_items,
    reason = "testnamnen är dokumentationen"
)]
mod tests {
    use super::{to_san, to_swedish};

    #[test]
    fn english_san_is_shown_in_swedish() {
        assert_eq!(to_swedish("Nf3"), "Sf3");
        assert_eq!(to_swedish("Bxc4+"), "Lxc4+");
        assert_eq!(to_swedish("Rfe1"), "Tfe1");
        assert_eq!(to_swedish("exd8=Q#"), "exd8=D#");
        assert_eq!(to_swedish("O-O-O"), "O-O-O");
        assert_eq!(to_swedish("Ke2"), "Ke2");
    }

    #[test]
    fn pawn_moves_pass_through() {
        assert_eq!(to_san("e4").as_deref(), Some("e4"));
        assert_eq!(to_san("exd5").as_deref(), Some("exd5"));
    }

    #[test]
    fn swedish_piece_letters_are_translated() {
        assert_eq!(to_san("Sf3").as_deref(), Some("Nf3"));
        assert_eq!(to_san("Lc4").as_deref(), Some("Bc4"));
        assert_eq!(to_san("Te1").as_deref(), Some("Re1"));
        assert_eq!(to_san("Dd4").as_deref(), Some("Qd4"));
        assert_eq!(to_san("Ke2").as_deref(), Some("Ke2"));
    }

    #[test]
    fn swedish_piece_words_are_translated() {
        assert_eq!(to_san("springare f3").as_deref(), Some("Nf3"));
        assert_eq!(to_san("Löpare c4").as_deref(), Some("Bc4"));
        assert_eq!(to_san("torn e1").as_deref(), Some("Re1"));
        assert_eq!(to_san("dam d4").as_deref(), Some("Qd4"));
        assert_eq!(to_san("kung e2").as_deref(), Some("Ke2"));
        assert_eq!(to_san("bonde e4").as_deref(), Some("e4"));
    }

    #[test]
    fn unknown_piece_word_is_rejected() {
        assert_eq!(to_san("häst f3"), None);
        assert_eq!(to_san("springare"), None);
        assert_eq!(to_san("springare f9"), None);
    }

    #[test]
    fn whitespace_is_normalised() {
        assert_eq!(to_san("  springare   f3 ").as_deref(), Some("Nf3"));
        assert_eq!(to_san(" e4 ").as_deref(), Some("e4"));
    }

    #[test]
    fn captures_checks_and_mates_are_kept() {
        assert_eq!(to_san("Sxe5").as_deref(), Some("Nxe5"));
        assert_eq!(to_san("Dxf7#").as_deref(), Some("Qxf7#"));
        assert_eq!(to_san("Lb5+").as_deref(), Some("Bb5+"));
        assert_eq!(to_san("springare xe5").as_deref(), Some("Nxe5"));
    }

    #[test]
    fn disambiguation_is_kept() {
        assert_eq!(to_san("Sbd2").as_deref(), Some("Nbd2"));
        assert_eq!(to_san("T1e2").as_deref(), Some("R1e2"));
        assert_eq!(to_san("T4e4").as_deref(), Some("R4e4"));
        assert_eq!(to_san("Dh4e1").as_deref(), Some("Qh4e1"));
        assert_eq!(to_san("Tfxe1").as_deref(), Some("Rfxe1"));
        assert_eq!(to_san("Sf3xe5").as_deref(), Some("Nf3xe5"));
    }

    #[test]
    fn castling_accepts_zeros_and_letters() {
        assert_eq!(to_san("0-0").as_deref(), Some("O-O"));
        assert_eq!(to_san("O-O").as_deref(), Some("O-O"));
        assert_eq!(to_san("0-0-0").as_deref(), Some("O-O-O"));
        assert_eq!(to_san("O-O-O").as_deref(), Some("O-O-O"));
    }

    #[test]
    fn promotion_letter_is_translated() {
        assert_eq!(to_san("e8=D").as_deref(), Some("e8=Q"));
        assert_eq!(to_san("exd8=S+").as_deref(), Some("exd8=N+"));
        assert_eq!(to_san("e8=T#").as_deref(), Some("e8=R#"));
    }

    #[test]
    fn promotion_to_king_or_unknown_piece_is_rejected() {
        assert_eq!(to_san("e8=K"), None);
        assert_eq!(to_san("e8=X"), None);
        assert_eq!(to_san("e8="), None);
    }

    #[test]
    fn malformed_moves_are_rejected() {
        assert_eq!(to_san(""), None);
        assert_eq!(to_san("e"), None);
        assert_eq!(to_san("4e"), None);
        assert_eq!(to_san("44"), None);
        assert_eq!(to_san("ee"), None);
        assert_eq!(to_san("e9"), None);
        assert_eq!(to_san("i4"), None);
        assert_eq!(to_san("4ee4"), None);
        assert_eq!(to_san("Sf3f3f3"), None);
        assert_eq!(to_san("Sf3?"), None);
        assert_eq!(to_san("Sf3++"), None);
        assert_eq!(to_san("Sf3+#"), None);
        assert_eq!(to_san("Sx"), None);
        assert_eq!(to_san("Sxf"), None);
        assert_eq!(to_san("Sx4f"), None);
        assert_eq!(to_san("Sxe5e5"), None);
        assert_eq!(to_san("S4exe5"), None);
        assert_eq!(to_san("Teee4"), None);
        assert_eq!(to_san("T44e4"), None);
        assert_eq!(to_san("Xe4"), None);
        assert_eq!(to_san("S"), None);
    }
}
