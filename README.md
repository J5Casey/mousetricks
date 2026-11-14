# Mouse Tricks

A **native** mod for [Horizon](https://github.com/arismoko/horizon) (the arismoko fork's mod API) that
makes the mouse quicker in inventories: drag with the right button to drop one item into each slot, drag with the left
button to gather a stack's item from many slots or to Shift-move many stacks at once, turn the wheel over a stack
to move its items one by one between your inventory and a container, and press **R** (or middle-click an empty spot)
to sort a chest or your inventory. Each trick can be turned off in game: Options > Mods > Mouse Tricks.

Inspired by Mouse Tweaks (https://modrinth.com/mod/mouse-tweaks); not affiliated, no code from it. Mouse Tricks was
written from scratch for Horizon from Mouse Tweaks' public description only.

> **Native mods are not sandboxed.** A native mod is Rust source compiled into your own Horizon executable and runs
> with your user rights. Read the source (it is small: `src/`) before building.

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

Where: every menu whose slots are all plain ones: the player's inventory screen (crafting grid and armour slots
included for the drags), chests, barrels, ender chests, shulker boxes, hoppers, dispensers and droppers, the crafting
table, the crafter, a horse's or llama's chest, in single player, on LAN and on servers. The mod knows these menus by
their type ids. A content mod's menu type (a crafting station, say) clicks as the game's own: the mod API gives a
client no way to ask what a mod menu's slots are for (see Compatibility). Screens with a machine's own slots (furnaces, brewing stands,
anvils, villagers, enchanting tables) and the creative inventory click exactly as before. The wheel over a crafting
grid's slot trades with your inventory (down sends one item back, up brings one more of it into the grid).

## Sorting

The sort works on one part of the screen, the one under the cursor:

- **a chest, barrel, ender chest or shulker box**: its slots;
- **your inventory** (its slots, the hotbar, the armour or the crafting grid, in the inventory screen or under a
  chest): the 27 main slots only. The hotbar joins them (after them, as drawn) only with **Sort the hotbar too**;
  the armour, the offhand and the crafting grid are never sorted;
- over the window's background, the part whose slots are nearest the cursor.

It does nothing with a stack in your hand, while the recipe book's search box has the keys, in the creative screen
(its item tabs and its inventory tab), and on screens with a machine's own slots: furnaces, anvils, villagers and the
like (there the key and the middle click are the game's). On a crafting table only your inventory sorts; a crafting
grid is never sorted.

**The order.** First a kind's partial stacks are merged: stacks merge only when they are the same item with the same
components, so items that differ in name, enchantments, damage or anything else stay apart. Then the stacks are
ordered, empty slots last, by the **Sort order** setting:

| Sort order | Key |
| --- | --- |
| creative tabs (default) | the item's place in the item registry (a mod can't read the creative tabs' order), then the name |
| name | the item's name, then the registry order |
| most items first | the kinds the section holds most of first, then the registry order, then the name |

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

**Options > Mods > Mouse Tricks** (its Config button) lists the settings; each applies at once. The sort's key is in
**Controls**, under Mouse Tricks (**Sort the inventory**, R by default); a mouse button works too (bound to the
middle button, a middle click sorts over a stack as well).

| Setting | Key | Values |
| --- | --- | --- |
| Right-drag: one item per slot | `right_drag` | ON, OFF |
| Left-drag: gather same items | `left_drag` | ON, OFF |
| Shift + left-drag: move stacks | `shift_drag` | ON, OFF |
| Wheel: move items one by one | `wheel` | ON, OFF |
| Wheel direction | `wheel_direction` | down sends, up brings back; up sends, down brings back; items follow the wheel (up over the inventory sends into the container above it, down over the container sends down into the inventory; the hotbar sends up into the main inventory) |
| Wheel brings back from | `wheel_search` | last slot, first slot: which end of the other side the wheel takes an item from first |
| Sort (key or middle click) | `sort` | ON, OFF |
| Sort order | `sort_order` | creative tabs (the item registry's order, see below), name, most items first |
| Sort the hotbar too | `sort_hotbar` | OFF, ON: sorting your inventory takes the hotbar too |

### Horizon's own inventory extra

Horizon has a built-in, off-by-default extra with similar controls (Options > Horizon Settings... > Extras... >
"Mouse Tweaks controls"), and one that sorts. Leave them off while Mouse Tricks is on.

## Compatibility

Mouse Tricks uses only the mod API (no game internals, no `[native.*]` rows), so it is not tied to one game build's
private functions. It is pinned (`horizon-mod.toml` `[native]`) to J5Casey/horizon `braid/ari-base` `86c2820370`
(the base every Braid mod pins).

Every item it moves is a `MenuRef::click`: the clicks a hand makes, predicted on your side and sent as ordinary
`ContainerClick`s. The hooks:

- `mouse_button_pre`: a left or right press starts the drag it may become; a middle click on an empty spot (or the
  sort key bound to a mouse button) sorts and is cancelled; a release ends the drag.
- `screen_mouse_button_pressed_pre` / `_released_pre`: a press holding a stack, which the game would make its own
  quick craft drag of, is taken from the game (with its release). The API has no way to drop the game's drag once
  the mod takes one over, so the mod follows the game's drag itself and, when the drag never becomes the mod's, its
  release does what the game's would (one click, the even spread, a double click's gather, a drop outside).
- `screen_mouse_dragged_pre`, `screen_mouse_scrolled_pre`: they get the game read-only, so the drag's and the
  wheel's clicks run at the next mutable moment (the next mouse button or `render_frame_pre`, the same frame).
- `render_frame_pre`: those clicks, and a paced sort's.
- `input_key`: the sort key (`screen_char_typed_pre` tells a key typed into a text box, the recipe book's search,
  apart from a command).
- `screen_render_post`: which menu is open (`ScreenView.menu`, its slot count and panel).

What the API doesn't give a client, and what Mouse Tricks does instead:

| Missing | Instead |
| --- | --- |
| What each slot of a content mod's menu is for (`MenuRef::slot_backing` is server-only) | Vanilla menus by type id; mod menus are left alone |
| The creative tabs' order | The item registry's order |
| A stack's shown name (an anvil name) | The item's translated name |
| Which keys vanilla's mappings are bound to | A hotbar key bound to a mouse button is no longer left alone |
| Whether the game's drag can be dropped | The press is taken from the game (above) |
| Shift + double click with a stack in hand (move all of a kind) | Not done |

## Build and try

Mouse Tricks is a flat native mod; the fork launcher compiles it into a custom game:

```sh
horizon-fork-launcher native-build <this folder> --trust-native-code
```

## Development

- `src/engine.rs`: everything the tricks decide (when a drag becomes the mod's, which slots take an item, the game's
  own drag for a press the mod takes, the wheel's targets and sources, the sort's section, order and click plan,
  its pace, the settings) as pure code over a small `Menu` trait, with unit tests against a fake menu that clicks
  as the game's menus do. A property test sorts 3000 random chests and inventories and checks that each ends sorted,
  merged, with every item kept, nothing clicked outside the section, the hand empty, and that sorting again takes no
  click.
- `src/lib.rs`: the glue to the mod API: the menu's roles by type id, the map between the engine's slot numbers and
  the menu's, and the hooks.

```sh
(cd engine && cargo test && cargo clippy --all-targets -- -D warnings)
# against a local checkout of the pinned base: mkdir -p .horizon/sdk && ln -s <checkout> .horizon/sdk/api
cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

MIT licensed, (c) 2026 d3grando.
