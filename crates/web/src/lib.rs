//! Webbgränssnittet: kopplar DOM och `localStorage` till app-lagret.
//!
//! Craten är medvetet tunn och undantagen från täckning och mutationstest, eftersom
//! den bara kan köras i en webbläsare. All logik ligger i `chess-trainer-app`, med
//! 100 % täckning. Kör `make serve` för att prova lokalt.

use std::cell::RefCell;

use chess_trainer_app::game::{Game, MoveError, Turn};
use chess_trainer_app::knowledge::{KnowledgeBase, advice};
use chess_trainer_app::{Color, Square, to_swedish, view};
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, Event, HtmlInputElement, Storage};

/// Nyckel för kunskapsbasen i `localStorage`.
const STORAGE_KEY: &str = "chess-trainer/kb";

/// Hit flyttas en kunskapsbas som inte gick att läsa, så att den aldrig skrivs över.
const BROKEN_KEY: &str = "chess-trainer/kb-trasig";

/// Så många rader av händelseloggen visas.
const LOG_LINES: usize = 8;

/// Allt gränssnittet håller i minnet.
struct State {
    /// Kunskapsbasen, speglad i `localStorage`.
    kb: KnowledgeBase,
    /// Pågående parti.
    game: Game,
    /// Rutan spelaren klickat på och vill flytta från.
    selected: Option<Square>,
    /// Händelselogg, senaste sist.
    log: Vec<String>,
    /// Får vi skriva till `localStorage`? Falskt om en trasig kunskapsbas inte kunde säkerhetskopieras.
    can_save: bool,
}

thread_local! {
    /// Gränssnittets tillstånd. Webbläsaren kör allt i en tråd, så en `RefCell` räcker.
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Felvärde till JavaScript.
fn js_error(message: &str) -> JsValue {
    JsValue::from_str(message)
}

/// Sidans dokument.
fn document() -> Result<Document, JsValue> {
    web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| js_error("dokument saknas"))
}

/// Webbläsarens `localStorage`, om den finns och är tillåten.
fn storage() -> Option<Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// Elementet med givet id.
fn by_id(doc: &Document, id: &str) -> Result<Element, JsValue> {
    doc.get_element_by_id(id).ok_or_else(|| js_error(id))
}

/// Läser kunskapsbasen. Returnerar också om det är säkert att skriva tillbaka.
fn load() -> (KnowledgeBase, bool) {
    let Some(store) = storage() else {
        return (KnowledgeBase::default(), false);
    };
    match store.get_item(STORAGE_KEY) {
        Ok(Some(json)) => KnowledgeBase::from_json(&json).map_or_else(
            |_| {
                (
                    KnowledgeBase::default(),
                    store.set_item(BROKEN_KEY, &json).is_ok(),
                )
            },
            |kb| (kb, true),
        ),
        Ok(None) => (KnowledgeBase::default(), true),
        Err(_) => (KnowledgeBase::default(), false),
    }
}

/// Sparar kunskapsbasen. Falskt om det inte gick.
fn save(state: &State) -> bool {
    state.can_save
        && match (storage(), state.kb.to_json()) {
            (Some(store), Ok(json)) => store.set_item(STORAGE_KEY, &json).is_ok(),
            _ => false,
        }
}

/// Ett frö till motorns slump. Bitarna i ett slumptal duger gott.
fn seed() -> u64 {
    js_sys::Math::random().to_bits()
}

/// Nu, som ISO 8601.
fn now() -> String {
    String::from(js_sys::Date::new_0().to_iso_string())
}

/// Ritar om allt från tillståndet.
fn render(state: &State, doc: &Document) -> Result<(), JsValue> {
    let flipped = state.game.player() == Color::Black;
    by_id(doc, "board")?.set_inner_html(&view::board_html(
        state.game.board(),
        flipped,
        state.selected,
    ));
    by_id(doc, "status")?.set_text_content(Some(&view::status_text(&state.game)));
    by_id(doc, "moves")?.set_text_content(Some(&view::move_list(state.game.moves())));
    let start = state.log.len().saturating_sub(LOG_LINES);
    let recent = state.log.get(start..).unwrap_or_default();
    by_id(doc, "log")?.set_text_content(Some(&recent.join("\n")));
    by_id(doc, "rating")?
        .set_text_content(Some(&format!("Uppskattad rating: {:.0}", state.kb.rating)));
    by_id(doc, "rating-chart")?.set_inner_html(&view::rating_svg(&state.kb));
    let lessons = by_id(doc, "lessons")?;
    lessons.set_inner_html("");
    for lesson in state.kb.lessons() {
        let item = doc.create_element("li")?;
        item.set_text_content(Some(&lesson));
        lessons.append_child(&item)?;
    }
    Ok(())
}

/// Kör `action` på tillståndet och ritar sedan om.
fn with_state(
    action: impl FnOnce(&mut State, &Document) -> Result<(), JsValue>,
) -> Result<(), JsValue> {
    let doc = document()?;
    STATE.with_borrow_mut(|slot| {
        let state = slot.as_mut().ok_or_else(|| js_error("inte startad"))?;
        action(state, &doc)?;
        render(state, &doc)
    })
}

/// Sparar partiet i kunskapsbasen när det är slut.
fn finish_if_over(state: &mut State) {
    if !state.game.is_over() {
        return;
    }
    if let Some(record) = state.game.take_record(now()) {
        state.kb.add(record);
        state.log.push(view::status_text(&state.game));
        if !save(state) {
            state
                .log
                .push("Kunde inte spara partiet i webbläsaren.".to_owned());
        }
    }
}

/// Loggar resultatet av ett drag.
fn report(state: &mut State, result: Result<Turn, MoveError>) {
    match result {
        Ok(turn) => {
            state.log.push(format!("Du: {}", to_swedish(&turn.played)));
            if let Some(mistake) = turn.mistake {
                state.log.push(format!(
                    "Aj: {} tappar ungefär {:.1} bönder. Bättre var {}. {}",
                    to_swedish(&mistake.played),
                    f64::from(mistake.loss) / 100.0,
                    to_swedish(&mistake.best),
                    advice(mistake.kind),
                ));
            }
            if let Some(reply) = turn.reply {
                state.log.push(format!("Motorn: {}", to_swedish(&reply)));
            }
        }
        Err(error) => state.log.push(error.to_string()),
    }
    finish_if_over(state);
}

/// Klick på brädet: första klicket väljer pjäs, andra flyttar den.
fn on_board_click(event: &Event) -> Result<(), JsValue> {
    let Some(target) = event.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
        return Ok(());
    };
    let Some(square) = target
        .closest("[data-sq]")?
        .and_then(|el| el.get_attribute("data-sq"))
        .and_then(|name| name.parse::<Square>().ok())
    else {
        return Ok(());
    };
    with_state(|state, _| {
        let own_piece = state.game.board().color_on(square) == Some(state.game.player());
        match state.selected {
            Some(from) if from != square && !own_piece => {
                state.selected = None;
                let result = state.game.play_squares(from, square);
                report(state, result);
            }
            Some(from) if from == square => state.selected = None,
            Some(_) | None => state.selected = own_piece.then_some(square),
        }
        Ok(())
    })
}

/// Draget i textfältet.
fn on_submit(event: &Event) -> Result<(), JsValue> {
    event.prevent_default();
    let doc = document()?;
    let input: HtmlInputElement = by_id(&doc, "move-input")?.dyn_into()?;
    let text = input.value();
    with_state(|state, _| {
        state.selected = None;
        let result = state.game.play(&text);
        if result.is_ok() {
            input.set_value("");
        }
        report(state, result);
        Ok(())
    })
}

/// Startar ett nytt parti. Ett pågående parti räknas som uppgivet.
fn new_game(player: Color) -> Result<(), JsValue> {
    with_state(|state, _| {
        if !state.game.is_over() && !state.game.moves().is_empty() {
            state.game.resign();
            finish_if_over(state);
        }
        state.game = Game::new(player, state.kb.rating, seed());
        state.selected = None;
        state
            .log
            .push(format!("Nytt parti. {}", view::status_text(&state.game)));
        if let Some(first) = state.game.moves().first() {
            state.log.push(format!("Motorn: {}", to_swedish(first)));
        }
        Ok(())
    })
}

/// En händelsehanterare.
type Handler = fn(&Event) -> Result<(), JsValue>;

/// Spelaren ger upp.
fn on_resign(_: &Event) -> Result<(), JsValue> {
    with_state(|state, _| {
        state.game.resign();
        finish_if_over(state);
        Ok(())
    })
}

/// Ställningen i ord, för blindspel.
fn on_describe(_: &Event) -> Result<(), JsValue> {
    with_state(|state, _| {
        state.log.push(state.game.describe());
        Ok(())
    })
}

/// Kopplar en händelse till en funktion. Lyssnaren lever lika länge som sidan.
fn listen(doc: &Document, id: &str, kind: &str, handler: Handler) -> Result<(), JsValue> {
    let callback = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        if let Err(error) = handler(&event) {
            web_sys::console::error_1(&error);
        }
    });
    by_id(doc, id)?.add_event_listener_with_callback(kind, callback.as_ref().unchecked_ref())?;
    callback.forget();
    Ok(())
}

/// Startpunkt när wasm-modulen laddats.
///
/// # Errors
/// Om sidan saknar något av de element gränssnittet behöver.
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let doc = document()?;
    let (kb, can_save) = load();
    let mut log = vec!["Välkommen! Skriv ett drag eller klicka på en pjäs.".to_owned()];
    if !can_save {
        log.push("Kunskapsbasen kan inte sparas i den här webbläsaren just nu.".to_owned());
    }
    let game = Game::new(Color::White, kb.rating, seed());
    let state = State {
        kb,
        game,
        selected: None,
        log,
        can_save,
    };
    render(&state, &doc)?;
    STATE.with_borrow_mut(|slot| *slot = Some(state));

    listen(&doc, "board", "click", on_board_click)?;
    listen(&doc, "move-form", "submit", on_submit)?;
    listen(&doc, "new-white", "click", |_| new_game(Color::White))?;
    listen(&doc, "new-black", "click", |_| new_game(Color::Black))?;
    listen(&doc, "resign", "click", on_resign)?;
    listen(&doc, "describe", "click", on_describe)?;
    Ok(())
}
