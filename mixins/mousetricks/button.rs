// native_hooks::Game::native_mouse_button_pre (a hook point, API 114): a
// window mouse button going down or up, on any screen, before the game
// dispatches it. Mouse Tricks notes the drag a left or right press may start
// (that press goes on to the game); a middle click on an empty spot (or the
// sort key bound to a mouse button) sorts, and the press is cancelled, as the
// game takes its own sort button's press as handled. A release ends the drag
// and goes on, and so does a press another mod already cancelled.
fn mousetricks_on_button(
    game: &mut crate::game::Game<'_>,
    button: i32,
    pressed: bool,
    canceled: &mut bool,
) {
    if !pressed || *canceled {
        game.mousetricks_release();
    } else if game.mousetricks_press(button) {
        *canceled = true;
    }
}
