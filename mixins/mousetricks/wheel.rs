// native_hooks::Game::native_screen_mouse_scrolled_pre_v2 (a hook point):
// the wheel over a screen, before anything scrolls. Cancelled when Mouse
// Tricks moved items (or would have, over a stack it works on); else the
// game's own handling goes on, a bundle under the cursor first.
fn mousetricks_on_wheel(game: &mut crate::game::Game<'_>, scroll: [f64; 2], canceled: &mut bool) {
    if !*canceled && game.mousetricks_wheel(scroll[1] as f32) {
        *canceled = true;
    }
}
