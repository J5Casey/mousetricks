// native_hooks::Game::native_screen_mouse_dragged_pre_v2 (a hook point): a
// held button moving over a screen, before any screen drag handler. A move
// that belongs to Mouse Tricks' drag is its own (cancelled, so the game's
// quick craft drag doesn't grow); any other goes on, and so does a move
// another mod already cancelled.
fn mousetricks_on_drag(game: &mut crate::game::Game<'_>, canceled: &mut bool) {
    if !*canceled && game.mousetricks_drag_move() {
        *canceled = true;
    }
}
