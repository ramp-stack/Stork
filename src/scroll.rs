use crate::canvas::Canvas;
use std::rc::Rc;
use std::cell::RefCell;

// ── ScrollConfig ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollConfig {
    pub accel:    f32,
    pub friction: f32,
    pub max_vel:  f32,
}

impl Default for ScrollConfig {
    fn default() -> Self { Self { accel: 5.5, friction: 0.10, max_vel: 90.0 } }
}

impl ScrollConfig {
    pub fn editor() -> Self { Self { accel: 2.5, friction: 0.18, max_vel: 40.0 } }
    pub fn fast()   -> Self { Self { accel: 9.0, friction: 0.08, max_vel: 150.0 } }
    pub fn slow()   -> Self { Self { accel: 3.0, friction: 0.15, max_vel: 45.0 } }
}

// ── AxisScroll ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct AxisScroll {
    pub offset:       f32,
    pub vel:          f32,
    pub intent:       f32,
    pub content_len:  f32,
    pub viewport_len: f32,
}

impl AxisScroll {
    fn new() -> Self {
        Self { offset: 0.0, vel: 0.0, intent: 0.0, content_len: 0.0, viewport_len: 0.0 }
    }

    fn push(&mut self, cfg: &ScrollConfig, units: f32) {
        let dir    = units.signum();
        let target = (units.abs() * cfg.accel).min(cfg.max_vel) * dir;
        let same   = self.vel.signum() == dir;
        self.vel   = if same && self.vel.abs() >= target.abs() { self.vel } else { target };
    }

    fn tick(&mut self, cfg: &ScrollConfig) {
        if self.intent != 0.0 { self.vel = self.intent; }
        self.offset += self.vel;
        self.vel    *= 1.0 - cfg.friction;
        if self.vel.abs() < 0.01 { self.vel = 0.0; }
        let max = (self.content_len - self.viewport_len).max(0.0);
        self.offset = self.offset.clamp(0.0, max);
    }
}

// ── ScrollState ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ScrollState {
    pub cfg: ScrollConfig,
    pub x:   AxisScroll,
    pub y:   AxisScroll,
}

impl ScrollState {
    pub fn new(cfg: ScrollConfig) -> Self {
        Self { cfg, x: AxisScroll::new(), y: AxisScroll::new() }
    }

    pub fn set_content_size(&mut self, w: f32, h: f32) {
        self.x.content_len  = w;
        self.y.content_len  = h;
    }

    pub fn set_viewport_size(&mut self, w: f32, h: f32) {
        self.x.viewport_len = w;
        self.y.viewport_len = h;
    }

    pub fn push_x(&mut self, units: f32) { self.x.push(&self.cfg, units); }
    pub fn push_y(&mut self, units: f32) { self.y.push(&self.cfg, units); }

    pub fn set_intent_x(&mut self, vel: f32) { self.x.intent = vel; }
    pub fn set_intent_y(&mut self, vel: f32) { self.y.intent = vel; }
    pub fn clear_intent(&mut self) { self.x.intent = 0.0; self.y.intent = 0.0; }

    pub fn set_vel_x(&mut self, v: f32) { self.x.vel = v; }
    pub fn set_vel_y(&mut self, v: f32) { self.y.vel = v; }
    pub fn vel_x(&self) -> f32 { self.x.vel }
    pub fn vel_y(&self) -> f32 { self.y.vel }
    pub fn max_vel(&self) -> f32 { self.cfg.max_vel }

    pub fn tick(&mut self) {
        self.x.tick(&self.cfg);
        self.y.tick(&self.cfg);
    }

    pub fn offset_x(&self) -> f32 { self.x.offset }
    pub fn offset_y(&self) -> f32 { self.y.offset }

    pub fn jump_to(&mut self, x: f32, y: f32) {
        let max_x = (self.x.content_len - self.x.viewport_len).max(0.0);
        let max_y = (self.y.content_len - self.y.viewport_len).max(0.0);
        self.x.offset = x.clamp(0.0, max_x);
        self.y.offset = y.clamp(0.0, max_y);
        self.x.vel    = 0.0;
        self.y.vel    = 0.0;
    }

    pub fn reset(&mut self) { self.jump_to(0.0, 0.0); }
}

// ── ScrollView ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ScrollView {
    state: Rc<RefCell<ScrollState>>,
    x:     Rc<RefCell<f32>>,
    y:     Rc<RefCell<f32>>,
    w:     Rc<RefCell<f32>>,
    h:     Rc<RefCell<f32>>,
}

impl ScrollView {
    pub fn new(x: f32, y: f32, w: f32, h: f32, cfg: ScrollConfig) -> Self {
        let mut state = ScrollState::new(cfg);
        state.set_viewport_size(w, h);
        Self {
            state: Rc::new(RefCell::new(state)),
            x:     Rc::new(RefCell::new(x)),
            y:     Rc::new(RefCell::new(y)),
            w:     Rc::new(RefCell::new(w)),
            h:     Rc::new(RefCell::new(h)),
        }
    }

    pub fn set_bounds(&self, x: f32, y: f32, w: f32, h: f32) {
        *self.x.borrow_mut() = x;
        *self.y.borrow_mut() = y;
        *self.w.borrow_mut() = w;
        *self.h.borrow_mut() = h;
        self.state.borrow_mut().set_viewport_size(w, h);
    }

    pub fn set_content_size(&self, content_w: f32, content_h: f32) {
        self.state.borrow_mut().set_content_size(content_w, content_h);
    }

    pub fn state(&self) -> &Rc<RefCell<ScrollState>> { &self.state }

    pub fn tick(&self)         { self.state.borrow_mut().tick(); }
    pub fn offset_x(&self) -> f32 { self.state.borrow().offset_x() }
    pub fn offset_y(&self) -> f32 { self.state.borrow().offset_y() }

    pub fn mount(&self, cv: &mut Canvas) {
        let state = Rc::clone(&self.state);
        let lx    = Rc::clone(&self.x);
        let ly    = Rc::clone(&self.y);
        let lw    = Rc::clone(&self.w);
        let lh    = Rc::clone(&self.h);

        cv.on_mouse_scroll(move |cv, (dx, dy)| {
            if let Some((mx, my)) = cv.mouse_position() {
                let ex = *lx.borrow(); let ey = *ly.borrow();
                let ew = *lw.borrow(); let eh = *lh.borrow();
                if mx < ex || mx > ex + ew || my < ey || my > ey + eh { return; }
            } else { return; }

            let mut st = state.borrow_mut();

            if dy != 0.0 {
                let dir    = if dy > 0.0 { 1.0f32 } else { -1.0 };
                let cfg    = st.cfg;
                let target = (dy.abs() * cfg.accel).min(cfg.max_vel) * dir;
                let cur    = st.y.vel;
                st.y.vel   = if cur.signum() == dir && cur.abs() >= target.abs() { cur } else { target };
            }

            if dx != 0.0 {
                let dir    = if dx > 0.0 { 1.0f32 } else { -1.0 };
                let cfg    = st.cfg;
                let target = (dx.abs() * cfg.accel).min(cfg.max_vel) * dir;
                let cur    = st.x.vel;
                let pos    = st.x.offset;
                let h_max  = (st.x.content_len - st.x.viewport_len).max(0.0);
                let new_v  = if cur.signum() == dir && cur.abs() >= target.abs() { cur } else { target };
                st.x.vel   = if pos <= 0.0 && new_v < 0.0 { 0.0 }
                        else if h_max > 0.0 && pos >= h_max && new_v > 0.0 { 0.0 }
                        else { new_v };
            }
        });
    }
}

// ── Canvas convenience ────────────────────────────────────────────────────────

impl Canvas {
    pub fn register_scroll_view(&mut self, view: &ScrollView) {
        view.mount(self);
    }
}