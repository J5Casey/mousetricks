//! Mouse Tricks for Horizon: inventory mouse drags, wheel moves and sorting,
//! as a native mod on the mod API (`horizon-mod-api`, lib `mod_api`).
//!
//! Everything the tricks decide is in `engine.rs`, which knows nothing of
//! Horizon. This file binds it to the open inventory screen: the menu's
//! slots, its carried stack and its clicks (`MenuRef`, the clicks a hand
//! makes, so a server sees nothing but ordinary `ContainerClick`s), and the
//! mouse and key hooks.
//!
//! Three things shape the glue:
//!
//! - The screen hooks that hear a drag and the wheel (`screen_mouse_dragged_pre`,
//!   `screen_mouse_scrolled_pre`) get the host read-only, and a click needs it
//!   mutable. They note what happened, and the clicks run at the next mutable
//!   moment: the next mouse button (`mouse_button_pre`) or the next frame
//!   (`render_frame_pre`), in the order the events came.
//! - The API has no way to drop the game's own quick craft drag once Mouse
//!   Tricks takes a drag over. So a press holding a stack, which the game
//!   would turn into its drag, is taken from the game at the press, and when
//!   the drag never becomes the mod's, its release does what the game's drag
//!   would have (`engine::QuickCraft`).
//! - The engine numbers slots as the game's inventory does (0..9 the hotbar,
//!   9..36 the main inventory, anything else the menu's own); the glue maps
//!   those to the menu's slot order and back (`Open`).
#![deny(unsafe_code)]

pub mod engine;

use engine::{
    Button, Click, Config, Drag, Kind, Layout, Menu as EngineMenu, Moment, Press, QuickCraft,
    Release, Setting, Sorting, Wheel,
};
use mod_api::{
    ClientHost, DragKind, Flow, Hooks, InputKeyEvent, ItemKey, ItemStack, KeyAction,
    KeyConflictContext, KeyRegistration, MenuClick, MenuRef, MouseButton, MouseButtonPreEvent,
    Registrar, RegistrationContext, RenderFramePreEvent, ScreenCharTypedPreEvent,
    ScreenMouseButtonPressedPreEvent, ScreenMouseButtonReleasedPreEvent,
    ScreenMouseDraggedPreEvent, ScreenMouseScrolledPreEvent, ScreenRenderPostEvent, ScreenView,
    SettingSpec, Tag,
};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

const ID: &str = engine::ID;
/// GLFW's shift bit in the input hooks' modifiers.
const SHIFT: u32 = 1;
/// Vanilla's double click: the same slot and button within this many ms.
const DOUBLE_CLICK_MS: u64 = 250;

/// What each slot of an open menu is for, as Mouse Tricks needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Hotbar(usize),
    Main(usize),
    Equipment,
    Storage,
    Grid,
    Result,
}

/// The roles of a menu's slots, in menu order, from its type id (None: the
/// player's own inventory) and slot count, when every slot is one Mouse
/// Tricks works with: the inventory, the crafting table, chests, barrels,
/// shulker boxes, ender chests, hoppers, dispensers, droppers, the crafter
/// and a mount's chest. None for any other menu (furnaces, anvils,
/// villagers, workstations, a content mod's menu type): there the screen
/// clicks as the game's own.
fn roles(menu: Option<&str>, slots: usize) -> Option<Vec<Role>> {
    let player = |roles: &mut Vec<Role>| {
        roles.extend((0..27).map(|i| Role::Main(9 + i)));
        roles.extend((0..9).map(Role::Hotbar));
    };
    let mut roles = Vec::with_capacity(slots);
    match menu {
        None => {
            roles.push(Role::Result);
            roles.extend([Role::Grid; 4]);
            roles.extend([Role::Equipment; 4]);
            player(&mut roles);
            roles.push(Role::Equipment);
        }
        Some("minecraft:crafting") => {
            roles.push(Role::Result);
            roles.extend([Role::Grid; 9]);
            player(&mut roles);
        }
        Some(m) if m.starts_with("minecraft:generic_9x") || m == "minecraft:shulker_box" => {
            roles.extend(std::iter::repeat_n(Role::Storage, slots.checked_sub(36)?));
            player(&mut roles);
        }
        Some("minecraft:hopper" | "minecraft:generic_3x3") => {
            roles.extend(std::iter::repeat_n(Role::Storage, slots.checked_sub(36)?));
            player(&mut roles);
        }
        Some("minecraft:crafter_3x3") => {
            roles.extend(std::iter::repeat_n(Role::Grid, slots.checked_sub(36)?));
            player(&mut roles);
        }
        Some(m) if m.starts_with("horizon:mount") => {
            roles.extend([Role::Equipment; 2]);
            roles.extend(std::iter::repeat_n(Role::Storage, slots.checked_sub(38)?));
            player(&mut roles);
        }
        Some(_) => return None,
    }
    (roles.len() == slots).then_some(roles)
}

/// The open inventory screen as the engine sees it: its menu, the roles of
/// its slots and the map between the engine's slot numbers and the menu's.
#[derive(Clone, Debug)]
struct Open {
    menu: MenuRef,
    roles: Vec<Role>,
    /// The screen's panel, `[x, y, width, height]` (outside it is "outside").
    rect: [f32; 4],
}

impl Open {
    /// The engine's number of menu slot `index`: the game's inventory
    /// numbers for the player's slots, 1000 + the index for the others.
    fn engine_slot(&self, index: usize) -> usize {
        match self.roles[index] {
            Role::Hotbar(n) | Role::Main(n) => n,
            _ => 1000 + index,
        }
    }
    /// The menu slot of the engine's number `slot`.
    fn menu_slot(&self, slot: usize) -> Option<usize> {
        if slot >= 1000 {
            return (slot - 1000 < self.roles.len()).then_some(slot - 1000);
        }
        self.roles
            .iter()
            .position(|r| matches!(r, Role::Hotbar(n) | Role::Main(n) if *n == slot))
    }
    fn with_role(&self, want: Role) -> Vec<usize> {
        (0..self.roles.len())
            .filter(|&i| self.roles[i] == want)
            .map(|i| self.engine_slot(i))
            .collect()
    }
    fn layout(&self) -> Layout {
        Layout {
            container: self.with_role(Role::Storage),
            grid: self.with_role(Role::Grid),
        }
    }
    /// The slot under `mouse` (menu order), as vanilla's `isHovering` finds
    /// it, and whether `mouse` is outside the window.
    fn target(&self, host: &dyn ClientHost, mouse: [f32; 2]) -> (Option<usize>, bool) {
        let [x, y, w, h] = self.rect;
        let outside = mouse[0] < x || mouse[1] < y || mouse[0] >= x + w || mouse[1] >= y + h;
        let slot = self.menu.slot_boxes(host).and_then(|boxes| {
            boxes.iter().position(|b| {
                b.is_some_and(|[bx, by, _, _]| {
                    mouse[0] >= bx - 1.0
                        && mouse[0] < bx + 17.0
                        && mouse[1] >= by - 1.0
                        && mouse[1] < by + 17.0
                })
            })
        });
        (slot, outside && slot.is_none())
    }
    /// The engine's number of the slot under `mouse`, never a result's.
    fn hovered(&self, host: &dyn ClientHost, mouse: [f32; 2]) -> Option<usize> {
        let slot = self.target(host, mouse).0?;
        (self.roles[slot] != Role::Result).then(|| self.engine_slot(slot))
    }
}

/// A drag or wheel move a read-only hook heard, for the next mutable moment.
#[derive(Clone, Debug)]
enum Pending {
    Moved { hovered: Option<usize>, shift: bool },
    Wheel { slot: usize, steps: i32 },
}

#[derive(Default)]
struct State {
    /// The open inventory screen as its last screen hook saw it.
    open: Option<Open>,
    drag: Option<Drag>,
    /// The game's drag, for a press Mouse Tricks took from the game.
    quick_craft: Option<QuickCraft>,
    /// The press was taken from the game: so is its release.
    taken: bool,
    /// The press and release the screen hooks cancel, set by
    /// `mouse_button_pre` for the same event.
    cancel_press: bool,
    cancel_release: bool,
    /// The last left press over a slot: its slot and when, for double clicks.
    last_left: Option<(usize, u64)>,
    wheel: Wheel,
    sorting: Option<Sorting<ItemStack>>,
    /// The last key's character went into a text box.
    typed_in_field: bool,
    pending: Vec<Pending>,
    /// Each item's place in the item registry, then its name, for sorting.
    order: HashMap<ItemKey, u64>,
    names: HashMap<ItemKey, String>,
}

#[derive(Default)]
struct MouseTricks {
    state: Mutex<State>,
}

impl MouseTricks {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }
}

fn config(host: &dyn ClientHost) -> Config {
    Config::read(|k| host.setting_bool(k), |k| host.setting_choice(k))
}

/// The player's open menu, while an inventory screen of one Mouse Tricks
/// works in is up and the mod is on.
fn open(host: &dyn ClientHost, state: &State) -> Option<Open> {
    let open = state.open.as_ref()?;
    let now = host.local_player()?.container_menu(host)?;
    (host.on(ID) && now == open.menu && open.menu.slot_boxes(host).is_some()).then(|| open.clone())
}

/// The game's name of a GLFW mouse button, as key mappings name it.
fn mouse_name(button: i32) -> String {
    match button {
        0 => "key.mouse.left".into(),
        1 => "key.mouse.right".into(),
        2 => "key.mouse.middle".into(),
        n => format!("key.mouse.{}", n + 1),
    }
}

fn sort_key(host: &dyn ClientHost) -> String {
    host.mod_key(ID, engine::SORT_KEY)
        .map_or_else(|| engine::SORT_KEY_DEFAULT.to_owned(), |(key, _)| key)
}

/// A component of `stack`'s own patch.
fn component<'a>(stack: &'a ItemStack, id: &str) -> Option<&'a Tag> {
    let patch = stack.components()?.as_nbt();
    patch
        .iter()
        .find(|(k, _)| k.to_string_lossy() == id)
        .map(|(_, v)| v)
}

/// The most of `stack` one slot holds: its own `max_stack_size`, else its
/// item's.
fn max_of(host: &dyn ClientHost, stack: &ItemStack) -> u32 {
    if let Some(Tag::Int(n)) = component(stack, "minecraft:max_stack_size") {
        return u32::try_from(*n).unwrap_or(1).max(1);
    }
    host.max_stack_size(stack.item()).unwrap_or(64).max(1)
}

fn is_bundle(host: &dyn ClientHost, stack: &ItemStack) -> bool {
    component(stack, "minecraft:bundle_contents").is_some()
        || host
            .item_default_component(stack.item(), "minecraft:bundle_contents")
            .is_some()
}

/// The open menu through the mod API's reads and clicks.
struct Live<'h> {
    host: &'h mut dyn ClientHost,
    open: &'h Open,
}

impl EngineMenu for Live<'_> {
    type Stack = ItemStack;
    fn slot(&mut self, slot: usize) -> Option<ItemStack> {
        let index = self.open.menu_slot(slot)?;
        self.open.menu.slot(&*self.host, index).flatten()
    }
    fn carried(&self) -> Option<ItemStack> {
        self.open.menu.carried(&*self.host).flatten()
    }
    fn count(&self, stack: &ItemStack) -> u32 {
        stack.count()
    }
    fn max(&self, stack: &ItemStack) -> u32 {
        max_of(&*self.host, stack)
    }
    fn same(&self, a: &ItemStack, b: &ItemStack) -> bool {
        a.is_stackable_with(b)
    }
    fn click(&mut self, slot: usize, click: Click) -> bool {
        let Some(slot) = self.open.menu_slot(slot) else {
            return false;
        };
        let click = match click {
            Click::Left => MenuClick::Pickup {
                slot: Some(slot),
                button: MouseButton::Left,
            },
            Click::Right => MenuClick::Pickup {
                slot: Some(slot),
                button: MouseButton::Right,
            },
            Click::QuickMove => MenuClick::QuickMove { slot },
        };
        self.open.menu.click(self.host, &[click])
    }
    /// Nothing to drop: a press the game would have made its drag of is the
    /// mod's from the start (see the module docs).
    fn take_over(&mut self) {}
}

/// The engine's slot reads without the clicks, for a read-only hook.
struct Reads<'h> {
    host: &'h dyn ClientHost,
    open: &'h Open,
}

impl EngineMenu for Reads<'_> {
    type Stack = ItemStack;
    fn slot(&mut self, slot: usize) -> Option<ItemStack> {
        let index = self.open.menu_slot(slot)?;
        self.open.menu.slot(self.host, index).flatten()
    }
    fn carried(&self) -> Option<ItemStack> {
        self.open.menu.carried(self.host).flatten()
    }
    fn count(&self, stack: &ItemStack) -> u32 {
        stack.count()
    }
    fn max(&self, stack: &ItemStack) -> u32 {
        max_of(self.host, stack)
    }
    fn same(&self, a: &ItemStack, b: &ItemStack) -> bool {
        a.is_stackable_with(b)
    }
    fn click(&mut self, _slot: usize, _click: Click) -> bool {
        false
    }
    fn take_over(&mut self) {}
}

impl MouseTricks {
    /// Runs what the read-only hooks noted, in order.
    fn flush(&self, host: &mut dyn ClientHost) {
        let mut st = self.state();
        let pending = std::mem::take(&mut st.pending);
        if pending.is_empty() {
            return;
        }
        let Some(open) = open(host, &st) else {
            st.drag = None;
            st.quick_craft = None;
            return;
        };
        let config = config(host);
        for p in pending {
            match p {
                Pending::Moved { hovered, shift } => {
                    let Some(mut drag) = st.drag.take() else {
                        continue;
                    };
                    let mut quick = st.quick_craft.take();
                    let now = Moment {
                        hovered,
                        shift,
                        vanilla: quick.as_ref().map(|q| q.slots().to_vec()),
                    };
                    let mut live = Live {
                        host: &mut *host,
                        open: &open,
                    };
                    let owned = drag.moved(&mut live, &now);
                    if let Some(q) = quick.as_mut() {
                        if owned {
                            // The drag is the mod's: the game's has nothing to do.
                            quick = None;
                        } else {
                            q.moved(&mut live, hovered);
                        }
                    }
                    st.drag = Some(drag);
                    st.quick_craft = quick;
                }
                Pending::Wheel { slot, steps } => {
                    let mut live = Live {
                        host: &mut *host,
                        open: &open,
                    };
                    engine::scroll(&mut live, &open.layout(), &config, slot, steps);
                }
            }
        }
    }

    /// A left or right press over an inventory screen. True when the press
    /// is taken from the game (the screen hook cancels it).
    fn press(&self, host: &mut dyn ClientHost, button: Button, shift: bool) -> bool {
        let mut st = self.state();
        let Some(open) = open(host, &st) else {
            return false;
        };
        let config = config(host);
        let hovered = open.hovered(host, host.cursor());
        let holding = open.menu.carried(host).flatten().is_some();
        let now = host.millis();
        let double = button == Button::Left
            && hovered.is_some()
            && st
                .last_left
                .is_some_and(|(slot, at)| Some(slot) == hovered && now - at < DOUBLE_CLICK_MS);
        if button == Button::Left {
            st.last_left = hovered.map(|s| (s, now));
        }
        let press = Press {
            button,
            slot: hovered,
            shift,
            holding,
        };
        let drag = Drag::start(&press, &config);
        st.drag = drag.clone();
        st.quick_craft = None;
        // A press holding a stack starts the game's quick craft drag: when
        // the mod's drag may take over, the press is the mod's, and the game's
        // drag is followed here instead.
        let (Some(slot), true, Some(_)) = (hovered, holding, drag) else {
            return false;
        };
        let mut live = Live {
            host: &mut *host,
            open: &open,
        };
        if double && !shift {
            // The double click's `PICKUP_ALL`, the game's own when it holds
            // the press: its release clicks nothing more.
            st.drag = None;
            st.last_left = None;
            if let Some(index) = open.menu_slot(slot) {
                open.menu
                    .click(live.host, &[MenuClick::PickupAll { slot: index }]);
            }
            return true;
        }
        st.quick_craft = Some(QuickCraft::start(&mut live, button, slot, shift));
        true
    }

    /// A mouse release: the drag ends. A press the mod took from the game
    /// gets the game's release (`QuickCraft::release`) unless the drag
    /// became the mod's. True when the release is the mod's.
    fn release(&self, host: &mut dyn ClientHost, shift: bool) -> bool {
        self.flush(host);
        let mut st = self.state();
        st.drag = None;
        let taken = std::mem::take(&mut st.taken);
        let Some(quick) = st.quick_craft.take() else {
            return taken;
        };
        let Some(open) = open(host, &st) else {
            return true;
        };
        let (target, outside) = open.target(host, host.cursor());
        let hovered = target
            .filter(|&s| open.roles[s] != Role::Result)
            .map(|s| open.engine_slot(s));
        let click = match quick.release(hovered, outside, shift) {
            Release::Spread(button, slots) => MenuClick::Drag {
                kind: match button {
                    Button::Left => DragKind::Split,
                    Button::Right => DragKind::One,
                },
                slots: slots.iter().filter_map(|&s| open.menu_slot(s)).collect(),
            },
            Release::Click(slot, click) => {
                let mut live = Live {
                    host: &mut *host,
                    open: &open,
                };
                live.click(slot, click);
                return true;
            }
            Release::Throw(button) => MenuClick::Pickup {
                slot: None,
                button: match button {
                    Button::Left => MouseButton::Left,
                    Button::Right => MouseButton::Right,
                },
            },
            Release::Nothing => return true,
        };
        open.menu.click(host, &[click]);
        true
    }

    /// The sort key, or a middle click (`middle`: only over an empty slot or
    /// no slot): sorts the part of the screen under the cursor with left
    /// clicks. True when the input was the sort's.
    fn sort(&self, host: &mut dyn ClientHost, middle: bool) -> bool {
        let mut st = self.state();
        let Some(open) = open(host, &st) else {
            return false;
        };
        let config = config(host);
        if !config.sort {
            return false;
        }
        let mouse = host.cursor();
        let (target, outside) = open.target(host, mouse);
        let holding = open.menu.carried(host).flatten().is_some();
        let over_stack = target.is_some_and(|s| open.menu.slot(host, s).flatten().is_some());
        if outside || holding || (middle && over_stack) {
            return false;
        }
        if st.sorting.is_some() {
            // One sort at a time: the one under way goes on.
            return true;
        }
        let container = open.with_role(Role::Storage);
        let hovered = target.map(|s| open.engine_slot(s));
        let near = hovered.is_none() && !container.is_empty() && near(host, &open, mouse);
        let slots = engine::sort_section(hovered, &container, near, config.sort_hotbar);
        let mut reads = Reads {
            host: &*host,
            open: &open,
        };
        let snapshot = engine::snapshot(&mut reads, &slots);
        let kinds: Vec<Kind> = snapshot
            .stacks
            .iter()
            .map(|s| kind(&*host, &mut st, s))
            .collect();
        let steps = engine::plan_sort(&snapshot.pieces, &kinds, config.sort_order);
        // On a server the clicks go out at a steady pace; in a world of the
        // window's own, all at once.
        let paced = !host
            .client_session()
            .client_server()
            .is_some_and(|s| s.integrated_local);
        let mut sorting = Sorting::new(slots, snapshot.stacks, steps, paced);
        let mut live = Live { host, open: &open };
        if sorting.run(&mut live, 0.0) {
            st.sorting = Some(sorting);
        }
        true
    }
}

/// The cursor is nearer the container's slots than the inventory's.
fn near(host: &dyn ClientHost, open: &Open, mouse: [f32; 2]) -> bool {
    let Some(boxes) = open.menu.slot_boxes(host) else {
        return false;
    };
    let nearest = |player: bool| {
        (0..open.roles.len())
            .filter(|&i| match open.roles[i] {
                Role::Hotbar(_) | Role::Main(_) => player,
                Role::Storage => !player,
                _ => false,
            })
            .filter_map(|i| boxes.get(i).copied().flatten())
            .map(|[x, y, _, _]| {
                let (dx, dy) = (x + 8.0 - mouse[0], y + 8.0 - mouse[1]);
                dx * dx + dy * dy
            })
            .fold(f32::INFINITY, f32::min)
    };
    nearest(false) < nearest(true)
}

/// What a sort knows of `stack`'s kind: its item's place in the item
/// registry, its name, its damage, whether it is a bundle (which stays
/// where it is).
fn kind(host: &dyn ClientHost, st: &mut State, stack: &ItemStack) -> Kind {
    if st.order.is_empty() {
        st.order = host
            .item_keys()
            .into_iter()
            .enumerate()
            .map(|(i, k)| (k, i as u64))
            .collect();
    }
    let item = stack.item();
    let name = st
        .names
        .entry(item)
        .or_insert_with(|| {
            let id = item.id(host);
            let (ns, path) = id.split_once(':').unwrap_or(("minecraft", &id));
            host.translate(&format!("item.{ns}.{path}"))
                .or_else(|| host.translate(&format!("block.{ns}.{path}")))
                .unwrap_or_else(|| path.to_owned())
        })
        .clone();
    Kind {
        max: max_of(host, stack),
        order: st.order.get(&item).copied().unwrap_or(u64::MAX),
        name,
        damage: stack.damage(),
        pinned: is_bundle(host, stack),
    }
}

#[mod_api::horizon_mod]
impl Hooks for MouseTricks {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        r.register_key_mapping(
            KeyRegistration::new(engine::SORT_KEY, engine::SORT_KEY_DEFAULT)
                .label("mousetricks.key.sort")
                .context(KeyConflictContext::Gui),
        );
        for setting in engine::SETTINGS {
            let spec = match setting {
                Setting::Toggle(key, default) => {
                    SettingSpec::toggle(key, format!("mousetricks.setting.{key}"), default)
                }
                Setting::Choice(key, options, default) => SettingSpec::choice(
                    key,
                    format!("mousetricks.setting.{key}"),
                    options
                        .iter()
                        .map(|o| format!("mousetricks.setting.{key}.{o}")),
                    default,
                ),
            };
            let key = spec.key.clone();
            if let Err(e) = r.register_setting(spec) {
                r.registration_refusal(format!("Mouse Tricks setting {key}: {e}"));
            }
        }
    }

    /// The open inventory screen, every frame it draws: what the input hooks
    /// that have no screen work on.
    fn screen_render_post(&self, host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>) {
        follow(&mut self.state(), host, event.screen);
    }

    /// A window mouse button going down or up, on any screen, before the
    /// game dispatches it. A left or right press over an inventory screen
    /// starts the drag it may become; a middle click on an empty spot, or
    /// the sort key bound to a mouse button, sorts and is cancelled. A
    /// release ends the drag.
    fn mouse_button_pre(&self, host: &mut dyn ClientHost, event: &mut MouseButtonPreEvent) -> Flow {
        let shift = event.modifiers & SHIFT != 0;
        {
            let mut st = self.state();
            st.cancel_press = false;
            st.cancel_release = false;
        }
        if !event.pressed {
            let taken = self.release(host, shift);
            self.state().cancel_release = taken;
            return Flow::Continue;
        }
        self.flush(host);
        {
            let mut st = self.state();
            st.drag = None;
            st.quick_craft = None;
            st.taken = false;
        }
        let taken = match event.button {
            0 => self.press(host, Button::Left, shift),
            1 => self.press(host, Button::Right, shift),
            other => {
                let key = mouse_name(other) == sort_key(host);
                if (key || other == 2) && self.sort(host, !key) {
                    return Flow::Cancel;
                }
                false
            }
        };
        let mut st = self.state();
        st.taken = taken;
        st.cancel_press = taken;
        Flow::Continue
    }

    fn screen_mouse_button_pressed_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenMouseButtonPressedPreEvent<'_>,
    ) -> Flow {
        let mut st = self.state();
        follow(&mut st, host, event.screen);
        if std::mem::take(&mut st.cancel_press) {
            return Flow::Cancel;
        }
        Flow::Continue
    }

    fn screen_mouse_button_released_pre(
        &self,
        _host: &dyn ClientHost,
        _event: &mut ScreenMouseButtonReleasedPreEvent<'_>,
    ) -> Flow {
        if std::mem::take(&mut self.state().cancel_release) {
            return Flow::Cancel;
        }
        Flow::Continue
    }

    /// The cursor moved with a button down: the drag hears of it at the
    /// next mutable moment.
    fn screen_mouse_dragged_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenMouseDraggedPreEvent<'_>,
    ) -> Flow {
        let mut st = self.state();
        if st.drag.is_none() {
            return Flow::Continue;
        }
        let Some(open) = open(host, &st) else {
            return Flow::Continue;
        };
        let hovered = open.hovered(host, event.screen.mouse);
        st.pending.push(Pending::Moved {
            hovered,
            shift: event.modifiers & SHIFT != 0,
        });
        Flow::Continue
    }

    /// The wheel over an inventory screen with an empty hand: over a stack
    /// with another side, it moves items one by one (and nothing else hears
    /// the wheel). A bundle keeps its own wheel.
    fn screen_mouse_scrolled_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenMouseScrolledPreEvent<'_>,
    ) -> Flow {
        let mut st = self.state();
        follow(&mut st, host, event.screen);
        let ready = open(host, &st)
            .filter(|o| config(host).wheel && o.menu.carried(host).flatten().is_none());
        let Some(open) = ready else {
            st.wheel.reset();
            return Flow::Continue;
        };
        let slot = open
            .hovered(host, event.screen.mouse)
            .filter(|&s| !open.layout().other_side(s).is_empty());
        let mut reads = Reads { host, open: &open };
        let bundle = slot
            .and_then(|s| reads.slot(s))
            .is_some_and(|s| is_bundle(host, &s));
        let Some(slot) = slot.filter(|_| !bundle) else {
            st.wheel.reset();
            return Flow::Continue;
        };
        let steps = st.wheel.steps(event.scroll[1] as f32);
        if steps != 0 {
            st.pending.push(Pending::Wheel { slot, steps });
        }
        Flow::Cancel
    }

    /// Runs what the read-only hooks noted; a drag or wheel left over from a
    /// screen that closed ends; a paced sort makes the clicks the time since
    /// the last frame allows, or stops when its screen is gone.
    fn render_frame_pre(&self, host: &mut dyn ClientHost, event: &mut RenderFramePreEvent) {
        self.flush(host);
        let mut st = self.state();
        let Some(open) = open(host, &st) else {
            st.drag = None;
            st.quick_craft = None;
            st.wheel.reset();
            st.sorting = None;
            return;
        };
        let Some(mut sorting) = st.sorting.take() else {
            return;
        };
        let mut live = Live { host, open: &open };
        if sorting.run(&mut live, event.elapsed.as_secs_f32()) {
            st.sorting = Some(sorting);
        }
    }

    /// The sort key over an inventory screen (repeats are left out).
    /// A typed character: whether it went into a text box (the recipe
    /// book's search), so its key is typing, not the sort key.
    fn screen_char_typed_pre(
        &self,
        _host: &dyn ClientHost,
        event: &mut ScreenCharTypedPreEvent<'_>,
    ) -> Flow {
        self.state().typed_in_field = event.field.is_some();
        Flow::Continue
    }

    fn input_key(&self, host: &mut dyn ClientHost, event: &mut InputKeyEvent<'_>) {
        // `input_key` comes after the key's characters were typed.
        let typed = std::mem::take(&mut self.state().typed_in_field);
        if event.action == KeyAction::Press && !typed && event.key == sort_key(host) {
            self.flush(host);
            self.sort(host, false);
        }
    }
}

/// Notes the open screen: an inventory screen of a menu Mouse Tricks works
/// in, or nothing.
fn follow(st: &mut State, host: &dyn ClientHost, screen: &ScreenView<'_>) {
    let menu = (screen.id == "inventory")
        .then(|| host.local_player()?.container_menu(host))
        .flatten();
    st.open = menu.and_then(|menu| {
        Some(Open {
            menu,
            roles: roles(screen.menu, screen.slots)?,
            rect: screen.rect,
        })
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_of_the_plain_menus() {
        let inv = roles(None, 46).unwrap();
        assert_eq!(inv[0], Role::Result);
        assert_eq!(inv[9], Role::Main(9));
        assert_eq!(inv[36], Role::Hotbar(0));
        assert_eq!(inv[45], Role::Equipment);
        let chest = roles(Some("minecraft:generic_9x3"), 63).unwrap();
        assert_eq!(chest[26], Role::Storage);
        assert_eq!(chest[27], Role::Main(9));
        assert_eq!(chest[62], Role::Hotbar(8));
        let table = roles(Some("minecraft:crafting"), 46).unwrap();
        assert_eq!(table[9], Role::Grid);
        assert_eq!(table[10], Role::Main(9));
        assert!(roles(Some("minecraft:furnace"), 39).is_none());
        assert!(roles(Some("minecraft:generic_9x3"), 20).is_none());
        let mount = roles(Some("horizon:mount5"), 53).unwrap();
        assert_eq!(
            &mount[..3],
            &[Role::Equipment, Role::Equipment, Role::Storage]
        );
        assert_eq!(mount[17], Role::Main(9));
    }
}
