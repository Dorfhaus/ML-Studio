pub mod convolver;
pub mod impulse;
mod specs;
pub mod vocoder;
use os_ui::{Canvas, Color, Font, Rect};
fn text(c: &mut Canvas, x: f32, y: f32, s: &str) {
    c.text(Font::new(12), x, y, Color::rgb(0xdddddd), s);
}
fn button(c: &mut Canvas, r: Rect, s: &str) {
    c.fill_rect(r, Color::rgb(0x343434));
    text(c, r.x + 10., r.y + 7., s);
}
#[cfg(test)]
mod tests;
