use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlElement, KeyboardEvent, Performance, Window};
use js_sys::Math;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    APP.with(|app| app.borrow_mut().init().expect("init failed"));
}

thread_local! {
    static APP: Rc<RefCell<App>> = Rc::new(RefCell::new(App::new()));
}

struct App {
    window: Window,
    document: Document,
    performance: Performance,
    root: HtmlElement,
    btn_start: HtmlElement,
    btn_reset: HtmlElement,
    progress: HtmlElement,
    stats: HtmlElement,
    // Overlay UI
    overlay: HtmlElement,
    overlay_time: HtmlElement,
    overlay_affirm: HtmlElement,
    btn_again: HtmlElement,

    target: u32,
    count: u32,
    running: bool,
    start_ts: f64,
    last_total_secs: f64,
    // Keypress race logic
    pressed: HashSet<String>,
    // Keep listeners alive
    keydown_closure: Option<Closure<dyn FnMut(KeyboardEvent)>>,
    keyup_closure: Option<Closure<dyn FnMut(KeyboardEvent)>>,
}

impl App {
    fn new() -> Self {
        let window = web_sys::window().expect("no window");
        let document = window.document().expect("no document");
        let performance = window.performance().expect("no performance");

        // Query elements up front (avoid borrowing closure on `document`)
        let root = document
            .query_selector("#app").unwrap().unwrap()
            .dyn_into::<HtmlElement>().unwrap();
        let btn_start = document
            .query_selector("#btn-start").unwrap().unwrap()
            .dyn_into::<HtmlElement>().unwrap();
        let btn_reset = document
            .query_selector("#btn-reset").unwrap().unwrap()
            .dyn_into::<HtmlElement>().unwrap();
        let progress = document
            .query_selector("#progress").unwrap().unwrap()
            .dyn_into::<HtmlElement>().unwrap();
        let stats = document
            .query_selector("#stats").unwrap().unwrap()
            .dyn_into::<HtmlElement>().unwrap();

        let overlay = document
            .query_selector("#done-overlay").unwrap().unwrap()
            .dyn_into::<HtmlElement>().unwrap();
        let overlay_time = document
            .query_selector("#overlay-time").unwrap().unwrap()
            .dyn_into::<HtmlElement>().unwrap();
        let overlay_affirm = document
            .query_selector("#overlay-affirmation").unwrap().unwrap()
            .dyn_into::<HtmlElement>().unwrap();
        let btn_again = document
            .query_selector("#btn-again").unwrap().unwrap()
            .dyn_into::<HtmlElement>().unwrap();

        Self {
            window,
            document,
            performance,
            root,
            btn_start,
            btn_reset,
            progress,
            stats,
            overlay,
            overlay_time,
            overlay_affirm,
            btn_again,
            target: 1000,
            count: 0,
            running: false,
            start_ts: 0.0,
            last_total_secs: 0.0,
            pressed: HashSet::new(),
            keydown_closure: None,
            keyup_closure: None,
        }
    }

    fn init(&mut self) -> Result<(), JsValue> {
        self.render();
        self.hide_overlay();

        // Start (use event listener API)
        let start_cb = {
            Closure::<dyn FnMut()>::new(move || {
                APP.with(|app| app.borrow_mut().start_race().unwrap());
            })
        };
        self.btn_start
            .add_event_listener_with_callback("click", start_cb.as_ref().unchecked_ref())?;
        start_cb.forget();

        // Reset (use event listener API)
        let reset_cb = {
            Closure::<dyn FnMut()>::new(move || {
                APP.with(|app| app.borrow_mut().reset().unwrap());
            })
        };
        self.btn_reset
            .add_event_listener_with_callback("click", reset_cb.as_ref().unchecked_ref())?;
        reset_cb.forget();

        // Play again button on overlay
        let again_cb = {
            Closure::<dyn FnMut()>::new(move || {
                APP.with(|app| {
                    let mut a = app.borrow_mut();
                    a.hide_overlay();
                    a.reset().ok();
                    a.start_race().ok();
                });
            })
        };
        self.btn_again
            .add_event_listener_with_callback("click", again_cb.as_ref().unchecked_ref())?;
        again_cb.forget();

        // Keydown handler: count on first transition to down (no auto-repeat)
        let kd = {
            Closure::<dyn FnMut(KeyboardEvent)>::new(move |evt: KeyboardEvent| {
                APP.with(|app| app.borrow_mut().on_keydown(evt));
            })
        };
        self.window
            .add_event_listener_with_callback("keydown", kd.as_ref().unchecked_ref())?;
        self.keydown_closure = Some(kd);

        // Keyup handler: release the key so a new cycle can count
        let ku = {
            Closure::<dyn FnMut(KeyboardEvent)>::new(move |evt: KeyboardEvent| {
                let key = evt.key(); // use evt.code() for physical key identity
                APP.with(|app| {
                    let mut a = app.borrow_mut();
                    a.pressed.remove(&key);
                });
            })
        };
        self.window
            .add_event_listener_with_callback("keyup", ku.as_ref().unchecked_ref())?;
        self.keyup_closure = Some(ku);

        // Clear potentially stuck keys on window blur
        let blur = {
            Closure::<dyn FnMut()>::new(move || {
                APP.with(|app| app.borrow_mut().pressed.clear());
            })
        };
        self.window
            .add_event_listener_with_callback("blur", blur.as_ref().unchecked_ref())?;
        blur.forget();

        Ok(())
    }

    fn start_race(&mut self) -> Result<(), JsValue> {
        if self.running {
            return Ok(());
        }
        self.count = 0;
        self.start_ts = 0.0;
        self.running = true;
        self.pressed.clear();
        self.hide_overlay();
        self.render();
        Ok(())
    }

    fn finish_race(&mut self) {
        self.running = false;
        let total_secs = (self.performance.now() - self.start_ts) / 1000.0;
        self.show_overlay(total_secs.max(0.0));
        self.render();
    }

    fn reset(&mut self) -> Result<(), JsValue> {
        self.count = 0;
        self.start_ts = 0.0;
        self.running = false;
        self.pressed.clear();
        self.hide_overlay();
        self.render();
        Ok(())
    }

    fn on_keydown(&mut self, evt: KeyboardEvent) {
        if !self.running {
            return;
        }
        // Ignore browser-generated repeats; we want explicit press→release cycles
        if evt.repeat() {
            return;
        }

        let key = evt.key(); // or evt.code()
        if self.pressed.contains(&key) {
            // already held; wait for keyup before counting again
            return;
        }
        self.pressed.insert(key);

        if self.start_ts == 0.0 {
            self.start_ts = self.performance.now();
        }

        self.count = self.count.saturating_add(1);
        if self.count >= self.target {
            self.finish_race();
        } else {
            self.render_progress_only();
        }
    }

    fn elapsed_secs(&self) -> f64 {
        if self.start_ts == 0.0 {
            0.0
        } else {
            let now = self.performance.now();
            ((now - self.start_ts).max(0.0)) / 1000.0
        }
    }

    fn render(&self) {
        self.btn_start
            .set_inner_text(if self.running { "Racing..." } else { "Start" });

        // Boolean `disabled`: presence disables; remove to enable
        if self.running {
            self.btn_start.set_attribute("disabled", "").ok();
            self.btn_reset.set_attribute("disabled", "").ok();
        } else {
            self.btn_start.remove_attribute("disabled").ok();
            self.btn_reset.remove_attribute("disabled").ok();
        }

        self.update_progress_and_stats();
    }

    fn render_progress_only(&self) {
        self.update_progress_and_stats();
    }

    fn update_progress_and_stats(&self) {
        let pct = (self.count as f64 / self.target as f64) * 100.0;
        self.progress
            .style()
            .set_property("--pct", &format!("{pct}%"))
            .ok();

        let elapsed = self.elapsed_secs();
        let kps = if elapsed > 0.0 {
            self.count as f64 / elapsed
        } else {
            0.0
        };

        let mut msg = format!(
            "Count: {}/{} • {:.1}% • {:.2} s • {:.1} keys/s",
            self.count,
            self.target,
            pct,
            elapsed,
            kps
        );

        if !self.running && self.count >= self.target {
            // final time (freeze)
            let total_secs = (self.performance.now() - self.start_ts) / 1000.0;
            let final_kps = self.target as f64 / total_secs.max(0.000_001);
            msg = format!(
                "Finished! {} keypresses in {:.2} s • {:.1} keys/s",
                self.target, total_secs, final_kps
            );
        }

        self.stats.set_inner_text(&msg);
    }

    fn show_overlay(&mut self, seconds: f64) {
        self.last_total_secs = seconds;
        self.overlay_time.set_inner_text(&format!("{:.3}", seconds));
        let affirmations = [
            "Lightning fingers!",
            "You are speed!",
            "Keyboard ninja!",
            "Blistering!",
            "Ridiculous!",
            "Godlike!",
            "Insane pace!",
            "Turbo mode!",
        ];
        let idx = (Math::random() * affirmations.len() as f64).floor() as usize;
        let pick = affirmations[idx.min(affirmations.len() - 1)];
        self.overlay_affirm.set_inner_text(pick);
        self.overlay.set_attribute("aria-hidden", "false").ok();
let _ = self.overlay.style().remove_property("display");
    }

    fn hide_overlay(&self) {
        self.overlay.set_attribute("aria-hidden", "true").ok();
let _ = self.overlay.style().remove_property("display");
    }
}