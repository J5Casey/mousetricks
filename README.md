# Mouse Tricks

A **native** [Horizon](https://github.com/horizon-rust/horizon) mod (a horizon-mixins source bundle, `.hnmod`) that
makes the mouse quicker in inventories: drag with the right button to drop one item into each slot, drag with the left
button to gather a stack's item from many slots or to Shift-move many stacks at once, turn the wheel over a stack
to move its items one by one between your inventory and a container, and press **R** (or middle-click an empty spot)
to sort a chest or your inventory. Each trick can be turned off in game: Options > Mods > Mouse Tricks > Settings.

Inspired by Mouse Tweaks (https://modrinth.com/mod/mouse-tweaks); not affiliated, no code from it. Mouse Tricks was
written from scratch for Horizon from Mouse Tweaks' public description only.

> **Native mods are not sandboxed.** Unlike `.hmod` (Wasm) mods, a native mod is Rust source compiled into your own
> Horizon executable and runs with your user rights. Read the source (it is small: `mixins/mousetricks/`) before
> building.

## What it does

Every trick is made of **ordinary slot clicks**, the very clicks your hand would make (left click, right click, Shift
click), sent through the game's own container click path. In a world of this game's own they run on the game's
verified menu engine; on a server (or a LAN host) each one is a normal `ContainerClick`, predicted on your side and
checked by the server like any click. Nothing is moved that a player couldn't move by clicking, so servers accept it,
and Mouse Tricks needs nothing on the server.

| Trick | How | Setting |
| --- | --- | --- |
| **Right-drag** | Holding a stack, press the right button over a slot and drag: as soon as the cursor leaves the first slot, it and every slot you enter get one item, at once (not at the release, as in vanilla). Entering a slot again puts one more in it. Slots holding another item are skipped (never swapped); full slots too. A right click that stays on its slot is the game's own click (one item, or a swap). | `rightDrag` |
| **Left-drag: gather** | With a stack in hand (or one the press just picked up), drag with the left button over slots of the **same item**: each one joins the stack in your hand, as long as all of it fits. Hold **Shift** while dragging and those slots are Shift-clicked (moved to the other side) instead. If your drag first spreads over two slots the vanilla way (to split the stack evenly), it stays vanilla's. | `leftDrag` |
| **Shift + left-drag: move** | With an empty hand, hold Shift, press the left button and drag: every stack you pass over is Shift-clicked, once per drag. Let go of Shift to pause. | `shiftDrag` |
| **Wheel** | With an empty hand, turn the wheel over a stack: **down** sends one of its items to the other side (onto a stack of it with room, else into the first empty slot, as a Shift click fills them), **up** brings one of the same item back into the stack. The other side is the container when one is open; in your own inventory, the hotbar and the main inventory trade. A bundle under the cursor keeps its own wheel. | `wheel` |
| **Sort** | With an empty hand, press the sort key (**R** by default) or **middle-click** an empty slot or the window's background: the part of the screen under the cursor is sorted (see below). A middle click on a stack is still the game's (pick block in creative). | `sort` |

Where: the player's inventory screen (crafting grid and armour slots included for the drags) and the chest-like
containers whose slots are all plain ones (the same set Horizon's own inventory extra works in): chests, barrels,
ender chests and shulker boxes, in single player (whose window is a client of its own server thread), on LAN and on
servers; with `--in-frame` single player also hoppers, dispensers and the crafting table. Other screens (furnaces,
anvils, villagers, a horse's inventory, the creative inventory) click exactly as before.

## Sorting

The sort works on one part of the screen, the one under the cursor:

- **a chest, barrel, ender chest or shulker box**: its slots;
- **your inventory** (its slots, the hotbar, the armour or the crafting grid, in the inventory screen or under a
  chest): the 27 main slots only. The hotbar joins them (after them, as drawn) only with **Sort the hotbar too**;
  the armour, the offhand and the crafting grid are never sorted;
- over the window's background, the part whose slots are nearest the cursor.

It does nothing with a stack in your hand, while the recipe book's search box has the keys, in the creative screen
(its item tabs and its inventory tab), and on screens that aren't plain storage: furnaces, crafting tables, anvils,
hoppers, dispensers, a horse's inventory and the like (there the key and the middle click are the game's).

**The order.** First a kind's partial stacks are merged: stacks merge only when they are the same item with the same
components, so items that differ in name, enchantments, damage or anything else stay apart. Then the stacks are
ordered, empty slots last, by the **Sort order** setting:

| Sort order | Key |
| --- | --- |
| creative tabs (default) | the item's place in the creative tabs (the item registry's order after them), then the name |
| name | the name (as shown: an anvil name counts), then the creative tabs |
| most items first | the kinds the section holds most of first, then the creative tabs, then the name |

Ties go to the kind with more items in the section, then the less damaged, then the one met first; a kind's full
stacks come before its last, partial one. **Bundles stay in their slots** (a click with a stack on a bundle, or with
a bundle on a stack, would put the stack into the bundle), and the rest is sorted around them.

**How.** Like every trick, a sort is made of ordinary left clicks (`PICKUP`) on the sorted slots only, the clicks a
hand makes: pick a stack up, put it down where its place is (taking up the stack that was there, which goes on to
its own place), merge into a stack of its kind. The whole sort is planned before the first click, against a model
of the game's click, and the plan is checked to end on exactly the sorted arrangement with every item kept and the
hand empty; a plan that wouldn't is dropped. Each stack already in a slot of its kind stays; moving the others
costs about one click per stack, plus one per chain of moves. A sorted section takes no click.

In a world of your own the clicks run at once. On a server (or as a LAN guest) each is a normal predicted
`ContainerClick`, and they go out at no more than 8 per game tick (160 a second), in steps that each start and end
with an empty hand; before each step Mouse Tricks checks that the slots still hold what the plan expects (nothing
picked up, no correction from the server, no click of yours in between), and stops there if not, so a sort never
leaves a stack in your hand. A second press while a sort is under way does nothing.

## Settings

**Options > Mods** lists Mouse Tricks with its **Settings** button. The row's own button turns the whole mod off (it
then does nothing at all: every hook passes through until the launcher's next build leaves it out) and on again. On
the settings page each click moves a button to its next value; it applies at once.

| Button | Key | Values a click cycles through |
| --- | --- | --- |
| Right-drag: one item per slot: ON | `rightDrag` | ON, OFF |
| Left-drag: gather same items: ON | `leftDrag` | ON, OFF |
| Shift + left-drag: move stacks: ON | `shiftDrag` | ON, OFF |
| Wheel: move items one by one: ON | `wheel` | ON, OFF |
| Wheel direction: down sends, up brings back | `wheelDirection` | `normal`; `inverted` (up sends, down brings back); `positional` (items follow the wheel: up over the inventory sends into the container above it, down over the container sends down into the inventory; the hotbar sends up into the main inventory). Shown while the wheel is on |
| Wheel brings back from: last slot | `wheelSearch` | `lastToFirst`, `firstToLast`: which end of the other side the wheel takes an item from first. Shown while the wheel is on |
| Sort (key or middle click): ON | `sort` | ON, OFF |
| Sort order: creative tabs | `sortOrder` | `registry` (creative tabs), `name`, `count` (most items first). Shown while the sort is on |
| Sort the hotbar too: OFF | `sortHotbar` | OFF, ON: sorting your inventory takes the hotbar too. Shown while the sort is on |
| Reset to defaults | | every setting back to its default |

The sort's **key** is the mod's key at the top of the page (**Sort the inventory**, R by default): click it and press
another key, as in Controls. A mouse button works too (bound to the middle button, a middle click sorts over a stack
as well).

The page writes `horizon-mods/.horizon-mod-data/mousetricks/config.json` (beside the game's `options.txt`):

```json
{
  "rightDrag": true,
  "leftDrag": true,
  "shiftDrag": true,
  "wheel": true,
  "wheelDirection": "normal",
  "wheelSearch": "lastToFirst",
  "sort": true,
  "sortOrder": "registry",
  "sortHotbar": false
}
```

Every key is optional. A file that doesn't parse (an unknown key, a wrong value) turns every trick off: the page shows
the error (click it to read the file again) and a reset button, and stderr gets `Mouse Tricks disabled: ...`.

### Horizon's own inventory extra

Horizon has a built-in, off-by-default extra with similar controls (Options > Horizon Settings... > Extras... >
"Mouse Tweaks controls"), and one that sorts. Leave them off
while Mouse Tricks is on (with both sorts on the same key, both sort, one after the other). If both are on, Mouse Tricks handles left drags and the wheel first, and Horizon's extra
keeps the right-drag (it takes the press before Mouse Tricks sees a drag), so the right-drag follows Horizon's rules,
not this mod's.

## Compatibility

Horizon's native mods can use **hook points** (`game/native_hooks.rs`, `NATIVE_HOOKS_API`): functions kept stable from
release to release, named in the source's `native-hooks.json` so a bundle hooks them by name (`hook = "frame"`) and keeps
working when they move. Mouse Tricks uses them for everything they cover:

- `mods`, `settings`, `settings_click`: the row in Options > Mods and the settings page;
  `native_setting` / `native_set_setting` / `native_on` / `native_window` for the file and the off switch;
- `frame`: a drag whose release never reached the inventory (the screen closed with the button down) is
  dropped; a paced sort makes its next clicks;
- `key`: the sort key (keys reach it on any screen); `NativeMod.keys` and `native_mod_key` for the key's
  binding on the settings page;
- `mouse_button_pre` (API 114, `InputEvent.MouseButton.Pre`): every window mouse button, pressed or released, on any
  screen, before the game dispatches it. A left or right press notes the drag it may start (the press goes on to the
  game); a middle click on an empty spot (or the sort key bound to a mouse button) sorts and is cancelled, as the
  game's own sort button answers "handled". A release ends the drag. A button the controls bind to a container key
  (a hotbar slot, swap with the offhand) is left to the game, as before.

The hook points that hear a drag and the wheel over a screen (`screen_mouse_dragged_pre`,
`screen_mouse_scrolled_pre`) take the game read-only, and Mouse Tricks clicks slots right there, so it still hooks two
game functions directly and is **pinned** to one Horizon commit:

| Hook | Kind | Why it is not a hook point |
| --- | --- | --- |
| `inventory_input::Game::inventory_drag_move` | HEAD, cancellable | The cursor moving over an inventory screen: a move that belongs to a Mouse Tricks drag clicks the slot it enters and is cancelled so the game's quick craft drag doesn't grow; any other move goes on. A drag whose button is no longer held ends here. |
| `tweaks::Game::tweak_scroll` | HEAD, cancellable | The wheel over an inventory screen, after a bundle had its turn: cancelled (answering "used") when Mouse Tricks moved items, else the game goes on. |

Every item Mouse Tricks moves goes through `Game::menu_click`, the game's own click path (no hook there). The sort
also reads the creative tabs' order (`Creative::order_of`), the shown name (`stations::hover_name`) and whether a
stack is a bundle (`bundle::Contents::of`).

Pinned: Horizon `36584650eaeec33553d41c4551a5b38b4a6cd0d7` (main, 2026-11-08), horizon-mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57` (0.6.0 and later resolve `hook` names), game 26.3. 0.2.1 follows two
changes on Horizon's main since 0.2.0: the inventory screen's mouse input moved to `game/inventory_input.rs`, and
`stations::hover_name` takes the pack's `Names` instead of a lookup closure. Checked for 0.2.1: `horizon-mixins apply`
and a build of the game on that commit and on main as of 2026-11-08 (`9e1abc64`).

## Build and try

Mouse Tricks is a source bundle; the launcher compiles it into a custom game:

```sh
horizon-launcher native-build <this folder> --version v0.2.97 --trust-native-code
```

`<this folder>` may also be a folder holding several native mods.

## Development

`mixins/mousetricks/`:

- `horizon-mixin.toml`: two support modules and eight HEAD hooks (above).
- `runtime.rs`: everything the tricks decide (when a drag becomes the mod's, which slots take an item, the wheel's
  targets and sources, the sort's section, order and click plan, its pace, the settings file and page) as pure code
  over a small `Menu` trait, with unit tests against a fake menu that clicks as the game's menus do (and fails on a
  click that would fill a bundle). A property test sorts 3000 random chests and inventories (27 and 54 slots, every
  order, merging kinds, items of one id with other names or damage, bundles) and checks that each ends sorted by
  keys computed apart from the plan, merged, with every item kept, nothing clicked outside the section, the hand
  empty, and that sorting again takes no click. It is both this repository's Cargo library and the injected
  `crate::game::mousetricks` module.
- `native.rs`: the adapter to the game (`crate::game::mousetricks_native`): the game's slots, carried stack and
  clicks, the sortable screens and what a sort knows of an item.
- `button.rs`, `drag.rs`, `wheel.rs`, `frame.rs`, `key.rs`, `list.rs`, `settings.rs`, `click.rs`: the hook
  helpers.

```sh
cargo test
cargo fmt --check
rustfmt --edition 2024 --check mixins/mousetricks/*.rs
cargo clippy --all-targets -- -D warnings
python tools/pack_hnmod.py   # dist/mousetricks-<version>.hnmod
```

MIT licensed, (c) 2026 d3grando.
