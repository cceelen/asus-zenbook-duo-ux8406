//! The display layout for an orientation of the laptop.
//!
//! This is `gnome-rotation/extension/rotation.js`.

use std::collections::BTreeMap;

use crate::geometry::{
    Area, DOWN, Monitor, State, area_of, find, index_of, is_rest, is_valid, mode_of, same,
    to_origin,
};

/// An orientation of the laptop, as the sensor gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    /// Upright.
    Normal,
    /// The left side is up.
    LeftUp,
    /// The right side is up.
    RightUp,
    /// Upside down.
    BottomUp,
    /// The sensor does not know.
    Undefined,
}

impl Orientation {
    /// The orientation for the text of the sensor. Other text is
    /// [`Orientation::Undefined`].
    #[must_use]
    pub fn from_sensor(text: &str) -> Self {
        match text {
            "normal" => Self::Normal,
            "left-up" => Self::LeftUp,
            "right-up" => Self::RightUp,
            "bottom-up" => Self::BottomUp,
            _ => Self::Undefined,
        }
    }

    /// The text of the sensor for this orientation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::LeftUp => "left-up",
            Self::RightUp => "right-up",
            Self::BottomUp => "bottom-up",
            Self::Undefined => "undefined",
        }
    }

    /// The orientation of a panel mounted upright for a transform; none for a
    /// mirrored panel.
    #[must_use]
    pub const fn from_transform(transform: u32) -> Option<Self> {
        match transform {
            0 => Some(Self::Normal),
            1 => Some(Self::LeftUp),
            2 => Some(Self::BottomUp),
            3 => Some(Self::RightUp),
            _ => None,
        }
    }

    /// The transform of a panel that is mounted upright, for the orientations
    /// that turn the panels (all but `Undefined`). A panel that is mounted
    /// turned has the turn of the panel in its transform too, and the state
    /// does not tell that one.
    #[must_use]
    pub const fn transform(self) -> Option<u32> {
        match self {
            Self::Normal => Some(0),
            Self::LeftUp => Some(1),
            Self::RightUp => Some(3),
            Self::BottomUp => Some(2),
            Self::Undefined => None,
        }
    }
}

/// A turn of the picture by 90 degrees.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// With the hands of a clock.
    Clockwise,
    /// Against the hands of a clock.
    Counterclockwise,
}

impl Direction {
    /// The direction for a text, or `None` for other text.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "clockwise" => Some(Self::Clockwise),
            "counterclockwise" => Some(Self::Counterclockwise),
            _ => None,
        }
    }
}

/// The control of the extension in the quick settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    /// No control: the desktop turns the panel itself, or there is no built-in
    /// panel.
    None,
    /// The switch "Auto-rotate", where a sensor gives the orientation.
    Switch,
    /// Two buttons that turn the panels, where no sensor gives it.
    Buttons,
}

impl Control {
    /// The name of the control.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Switch => "switch",
            Self::Buttons => "buttons",
        }
    }
}

/// The positions of the built-in panels that are shown, each by itself.
fn panels_of(state: &State) -> Vec<usize> {
    state
        .builtin
        .iter()
        .filter_map(|connector| index_of(&state.logical, connector))
        .filter(|&at| state.logical[at].connectors.len() == 1)
        .collect()
}

/// The panels in the order in which they lie on the laptop, from its top to its
/// bottom. The layout tells: upright, a panel further down in the layout is
/// further down on the laptop; turned, the layout follows the laptop.
///
/// The last panel tells how the layout is turned: the compositor turns the
/// first built-in panel by itself where it follows the orientation, and that
/// one only. The layout is then still the one of the other panels. The panels
/// are not empty, and their transforms are 0 to 3.
fn from_top(state: &State, panels: &[usize]) -> Vec<usize> {
    let (dx, dy) = DOWN[state.logical[panels[panels.len() - 1]].transform as usize];
    let mut order = panels.to_vec();

    order.sort_by_key(|&at| state.logical[at].x * dx + state.logical[at].y * dy);
    order
}

/// The layout from before the turn, if it still fits: the same monitors, each
/// at the same scale. Else `None`.
fn restored(state: &State, upright: Option<&[Monitor]>) -> Option<Vec<Monitor>> {
    let upright = upright.filter(|upright| is_rest(&state.logical, upright, None))?;
    let fits = upright.iter().all(|monitor| {
        monitor
            .connectors
            .first()
            .and_then(|connector| find(&state.logical, connector))
            .is_some_and(|now| same(now.scale, monitor.scale))
    });

    fits.then(|| upright.to_vec())
}

/// How the panels are to lie: the transform of each one, whether they lie
/// side by side (else one below the other), and the panels in the order from
/// the top left corner.
struct Plan<'a> {
    transform_of: &'a dyn Fn(usize) -> u32,
    across: bool,
    order: Vec<usize>,
}

/// Where a monitor beside the panels goes, from the monitor and its area.
type Mover<'a> = dyn Fn(&Monitor, &Area) -> (i32, i32) + 'a;

/// Panels put in a block from a corner.
struct Block {
    /// The top left corner of each panel, in the order of the plan.
    places: Vec<(usize, (i32, i32))>,
    /// The width of the whole.
    width: i32,
    /// The height of the whole.
    height: i32,
}

/// The place of a panel turned as the plan says, at the origin of the layout.
fn turned_area(state: &State, plan: &Plan<'_>, at: usize) -> Area {
    area_of(
        state,
        &Monitor {
            transform: (plan.transform_of)(at),
            ..state.logical[at].clone()
        },
    )
}

/// The panels of the plan from (x, y) on: side by side if `across`, else one
/// below the other.
fn block(state: &State, plan: &Plan<'_>, (x, y): (i32, i32)) -> Block {
    let mut places = Vec::new();
    let (mut width, mut height) = (0, 0);

    for &at in &plan.order {
        let area = turned_area(state, plan, at);

        if plan.across {
            places.push((at, (x + width, y)));
            width += area.width;
            height = height.max(area.height);
        } else {
            places.push((at, (x, y + height)));
            width = width.max(area.width);
            height += area.height;
        }
    }
    Block {
        places,
        width,
        height,
    }
}

/// `start` of a monitor of `length` along an edge, moved to the nearest panel
/// of `edge` if it misses all of them. The edge is a list of panels, each as
/// its start and its size along the edge.
fn slide(start: i32, length: i32, edge: &[(i32, i32)]) -> i32 {
    let gap =
        |&(from, size): &(i32, i32)| (from - (start + length)).max(start - (from + size)).max(0);

    if edge
        .iter()
        .any(|&(from, size)| start < from + size && start + length > from)
    {
        return start;
    }

    // The edge is never empty: the panel that makes the block as wide or as
    // high as it is reaches its edge.
    let &(from, size) = edge.iter().min_by_key(|panel| gap(panel)).expect("a panel");

    if start + length <= from {
        from
    } else {
        from + size - length
    }
}

/// All monitors in one row, in the order in which they are from left to
/// right. `together`: the panels as the block, else each by itself, in the
/// order of the plan.
fn in_row(state: &State, panels: &[usize], plan: &Plan<'_>, together: bool) -> Vec<Monitor> {
    let mut row: Vec<(i32, i32, u32)> = state
        .logical
        .iter()
        .map(|monitor| (monitor.x, monitor.y, monitor.transform))
        .collect();
    let mut waiting = plan.order.clone();
    let mut x = 0;
    let mut by_x: Vec<usize> = (0..state.logical.len()).collect();

    by_x.sort_by_key(|&at| state.logical[at].x);
    for at in by_x {
        if !panels.contains(&at) {
            row[at] = (x, 0, state.logical[at].transform);
            x += area_of(state, &state.logical[at]).width;
        } else if !together {
            let panel = waiting.remove(0);

            row[panel] = (x, 0, (plan.transform_of)(panel));
            x += turned_area(state, plan, panel).width;
        } else if !waiting.is_empty() {
            // The first panel from the left brings the others with it.
            let whole = block(state, plan, (x, 0));

            waiting.clear();
            for &(panel, (px, py)) in &whole.places {
                row[panel] = (px, py, (plan.transform_of)(panel));
            }
            x += whole.width;
        }
    }
    // The compositor wants the monitors in the order in which it has them.
    state
        .logical
        .iter()
        .zip(row)
        .map(|(monitor, (x, y, transform))| Monitor {
            x,
            y,
            transform,
            ..monitor.clone()
        })
        .collect()
}

/// The layouts for a plan, the best one first:
///
/// 1. The panels placed anew from their top left corner; a monitor that lies
///    beside them, right of them or below them, moves by as much as the panels
///    grow in that direction.
/// 2. The same, and a monitor that touched an edge of the old block also
///    touches the widest edge of the new one: it slides along that edge to the
///    nearest panel that reaches it. The block is not a rectangle when the
///    panels differ in size.
/// 3. The first one, with the other monitors left where they are.
/// 4. All monitors in one row, in the order in which they are from left to
///    right, the panels together as the block.
/// 5. The same, with each panel by itself in the row. The compositor can
///    always take this one.
///
/// `panels` is not empty.
fn placed(state: &State, panels: &[usize], plan: &Plan<'_>) -> [Vec<Monitor>; 5] {
    let before: Vec<Area> = panels
        .iter()
        .map(|&at| area_of(state, &state.logical[at]))
        .collect();
    let left = before.iter().map(|area| area.x).min().unwrap_or(0);
    let up = before.iter().map(|area| area.y).min().unwrap_or(0);
    let right = before
        .iter()
        .map(|area| area.x + area.width)
        .max()
        .unwrap_or(0);
    let down = before
        .iter()
        .map(|area| area.y + area.height)
        .max()
        .unwrap_or(0);
    let anew = block(state, plan, (left, up));
    let wider = anew.width - (right - left);
    let taller = anew.height - (down - up);
    let place_of = |at: usize| anew.places.iter().find(|(panel, _)| *panel == at);
    // Beside the panels: a monitor shares rows, or columns, with them.
    let sides = |monitor: &Monitor, area: &Area| {
        (
            monitor.x >= right && area.y < down && area.y + area.height > up,
            monitor.y >= down && area.x < right && area.x + area.width > left,
        )
    };
    let beside = |monitor: &Monitor, area: &Area| {
        let (on_right, below) = sides(monitor, area);

        (
            if on_right {
                monitor.x + wider
            } else {
                monitor.x
            },
            if below { monitor.y + taller } else { monitor.y },
        )
    };
    // The panels of the new block that reach its right and its bottom edge.
    let fresh: Vec<Area> = anew
        .places
        .iter()
        .map(|&(at, (x, y))| Area {
            x,
            y,
            ..turned_area(state, plan, at)
        })
        .collect();
    let reach_right: Vec<(i32, i32)> = fresh
        .iter()
        .filter(|area| area.x + area.width == left + anew.width)
        .map(|area| (area.y, area.height))
        .collect();
    let reach_bottom: Vec<(i32, i32)> = fresh
        .iter()
        .filter(|area| area.y + area.height == up + anew.height)
        .map(|area| (area.x, area.width))
        .collect();
    let fitted = |monitor: &Monitor, area: &Area| {
        let (on_right, below) = sides(monitor, area);
        let (x, y) = beside(monitor, area);

        (
            if below && monitor.y == down {
                slide(x, area.width, &reach_bottom)
            } else {
                x
            },
            if on_right && monitor.x == right {
                slide(y, area.height, &reach_right)
            } else {
                y
            },
        )
    };
    let with_others = |moved: Option<&Mover<'_>>| -> Vec<Monitor> {
        state
            .logical
            .iter()
            .enumerate()
            .map(|(at, monitor)| {
                let mut changed = monitor.clone();

                if let Some(&(_, (x, y))) = place_of(at) {
                    changed.x = x;
                    changed.y = y;
                    changed.transform = (plan.transform_of)(at);
                } else if let Some(moved) = moved {
                    (changed.x, changed.y) = moved(monitor, &area_of(state, monitor));
                }
                changed
            })
            .collect()
    };
    [
        with_others(Some(&beside)),
        with_others(Some(&fitted)),
        with_others(None),
        in_row(state, panels, plan, true),
        in_row(state, panels, plan, false),
    ]
    .map(|way| to_origin(&way))
}

/// Whether the panels cannot be turned: one is mirrored (transforms 4 to 7),
/// which is the user's own setting that a turn would take away, or a monitor
/// has no mode, so that it has no size.
fn unfit(state: &State, panels: &[usize]) -> bool {
    panels.iter().any(|&at| state.logical[at].transform > 3)
        || state
            .logical
            .iter()
            .any(|monitor| mode_of(state, monitor).is_none())
}

/// The panels in the order in which they lie on the laptop, from its top to
/// its bottom. Panels that are turned differently (by hand, one by one) have
/// no common turn that tells the order. Then `top` tells it, if it is given
/// and names all panels: the connectors from the top of the laptop to its
/// bottom, the order of the compositor's list of built-in panels. Else
/// [`from_top`] decides.
fn top_first(state: &State, panels: &[usize], top: Option<&[String]>) -> Vec<usize> {
    let connector = |at: usize| &state.logical[at].connectors[0];
    let alike = panels
        .iter()
        .all(|&at| state.logical[at].transform == state.logical[panels[0]].transform);

    match top.filter(|top| !alike && panels.iter().all(|&at| top.contains(connector(at)))) {
        Some(top) => {
            let mut order = panels.to_vec();

            order.sort_by_key(|&at| top.iter().position(|name| name == connector(at)));
            order
        }
        None => from_top(state, panels),
    }
}

/// The layout for an orientation of the laptop; `None` for nothing to do.
///
/// - Upright: the built-in panels one below the other.
/// - Left side up: the panels turned and side by side, the bottom one on the
///   left.
/// - Right side up: turned the other way, the bottom one on the right.
/// - Upside down: the panels turned by 180 degrees and one below the other,
///   the bottom one on top.
///
/// A laptop with one panel has that one turned. `upright` is the layout from
/// before the panels were turned: upright again, it is put back as it was if it
/// still fits. Else the panels are placed anew, in the first way that the
/// compositor can take. `top` is the order of the panels from the top of the
/// laptop, for panels that are turned differently (refer to [`top_first`]).
/// `None` if the orientation is another one, no built-in panel is shown by
/// itself, a panel is mirrored, a monitor has no mode, or the panels are turned
/// that way already: the place of the monitors is then not changed, whatever
/// it is.
#[must_use]
pub fn rotate(
    state: &State,
    orientation: Orientation,
    upright: Option<&[Monitor]>,
    top: Option<&[String]>,
) -> Option<Vec<Monitor>> {
    let panels = panels_of(state);
    let transform = orientation.transform()?;

    if unfit(state, &panels) {
        return None;
    }
    // Turned as the laptop is: the layout is the user's, or the compositor's
    // own, and is left alone. Also with no panel shown by itself: all of none.
    if panels
        .iter()
        .all(|&at| state.logical[at].transform == transform)
    {
        return None;
    }

    // The bottom panel first where the bottom of the laptop is at the left or
    // at the top.
    let mut order = top_first(state, &panels, top);

    if transform == 1 || transform == 2 {
        order.reverse();
    }

    let restored = if orientation == Orientation::Normal {
        restored(state, upright)
    } else {
        None
    };
    let plan = Plan {
        transform_of: &|_| transform,
        across: transform == 1 || transform == 3,
        order,
    };

    // Each of them turns a panel, so that none is the layout of now.
    restored
        .into_iter()
        .chain(placed(state, &panels, &plan))
        .find(|way| is_valid(state, way))
}

/// How the panels lie in the layout now: whether they are side by side (else
/// one below the other), and the panels in the order from the left or from the
/// top. It follows the middle of each panel: a panel that is turned has
/// another size than the one beside it, and still lies where it lies.
fn arrangement(state: &State, panels: &[usize]) -> (bool, Vec<usize>) {
    let middles: Vec<(i32, i32)> = panels
        .iter()
        .map(|&at| {
            let area = area_of(state, &state.logical[at]);

            (2 * area.x + area.width, 2 * area.y + area.height)
        })
        .collect();
    let spread = |axis: fn(&(i32, i32)) -> i32| {
        middles.iter().map(axis).max().unwrap_or(0) - middles.iter().map(axis).min().unwrap_or(0)
    };
    let across = spread(|middle| middle.0) > spread(|middle| middle.1);
    let mut order = panels.to_vec();

    order.sort_by_key(|&at| {
        if across {
            state.logical[at].x
        } else {
            state.logical[at].y
        }
    });
    (across, order)
}

/// The layout for built-in panels turned to orientations, the other panels as
/// they are: `held` has the orientation of each panel by its connector. `None`
/// for nothing to do. The panels stay as they lie: one below the other, or side
/// by side, in the same order. They are placed anew from their top left corner
/// with their new sizes, and the monitors beside them move as in [`rotate`]. An
/// entry for a connector that is not a built-in panel shown by itself (a panel
/// that is off or gone) is left out.
///
/// `None` if an orientation is unknown, a panel is mirrored, a monitor has no
/// mode, or each panel is in its orientation already.
#[must_use]
pub fn turn_many(state: &State, held: &BTreeMap<String, Orientation>) -> Option<Vec<Monitor>> {
    let panels = panels_of(state);
    let target = |at: usize| {
        held.get(&state.logical[at].connectors[0])
            .and_then(|orientation| orientation.transform())
            .unwrap_or(state.logical[at].transform)
    };

    if held
        .values()
        .any(|orientation| orientation.transform().is_none())
        || unfit(state, &panels)
        || panels
            .iter()
            .all(|&at| target(at) == state.logical[at].transform)
    {
        return None;
    }

    let (across, order) = arrangement(state, &panels);
    let plan = Plan {
        transform_of: &target,
        across,
        order,
    };

    placed(state, &panels, &plan)
        .into_iter()
        .find(|way| is_valid(state, way))
}

/// The layout for one built-in panel turned to an orientation ([`turn_many`]
/// for one panel). `None` also for a connector that is not a built-in panel
/// shown by itself.
#[must_use]
pub fn turn_one(state: &State, connector: &str, orientation: Orientation) -> Option<Vec<Monitor>> {
    turn_many(
        state,
        &BTreeMap::from([(connector.to_owned(), orientation)]),
    )
}

/// A built-in screen that can be turned on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Screen {
    /// Its connector.
    pub connector: String,
    /// Its name for the user.
    pub name: String,
    /// How it is turned; none for a mirrored panel.
    pub orientation: Option<Orientation>,
}

/// The built-in panels that can be turned one by one, in the order of the
/// compositor's list of built-in panels. The first of two is the upper screen
/// and the second the lower one, as on the UX8406.
#[must_use]
pub fn screens(state: &State) -> Vec<Screen> {
    let panels = panels_of(state);
    let count = state.builtin.len();

    state
        .builtin
        .iter()
        .enumerate()
        .filter_map(|(index, connector)| {
            let at = panels
                .iter()
                .copied()
                .find(|&at| state.logical[at].connectors[0] == *connector)?;
            let name = match count {
                1 => "Built-in screen".to_owned(),
                2 => ["Upper screen", "Lower screen"][index].to_owned(),
                _ => format!("Built-in screen {}", index + 1),
            };

            Some(Screen {
                connector: connector.clone(),
                name,
                orientation: Orientation::from_transform(state.logical[at].transform),
            })
        })
        .collect()
}

/// The layout to remember after [`rotate`] gave a layout for `state` and the
/// compositor took it: `state.logical` if that turn was away from upright, to
/// put it back as it was; nothing when the laptop is upright again; else the
/// one from before.
#[must_use]
pub fn upright_after(
    state: &State,
    orientation: Orientation,
    upright: Option<&[Monitor]>,
) -> Option<Vec<Monitor>> {
    if orientation == Orientation::Normal {
        return None;
    }
    if panels_of(state)
        .iter()
        .all(|&at| state.logical[at].transform == 0)
    {
        Some(state.logical.clone())
    } else {
        upright.map(<[Monitor]>::to_vec)
    }
}

/// The control of the extension in the quick settings.
///
/// - [`Control::None`]: the desktop turns the panel itself and shows its own
///   switch for the same setting, or there is no built-in panel.
/// - [`Control::Switch`]: a sensor gives the orientation of the laptop.
/// - [`Control::Buttons`]: no sensor gives it.
#[must_use]
pub fn control(managed: bool, sensor: bool, builtin: bool) -> Control {
    if managed || !builtin {
        Control::None
    } else if sensor {
        Control::Switch
    } else {
        Control::Buttons
    }
}

/// The orientation after one turn of the picture by 90 degrees. `None` where
/// the turn goes past left side up or right side up, or for another
/// orientation.
#[must_use]
pub fn turned(orientation: Orientation, direction: Direction) -> Option<Orientation> {
    match (orientation, direction) {
        (Orientation::LeftUp, Direction::Clockwise)
        | (Orientation::RightUp, Direction::Counterclockwise) => Some(Orientation::Normal),
        (Orientation::Normal, Direction::Clockwise) => Some(Orientation::RightUp),
        (Orientation::Normal, Direction::Counterclockwise) => Some(Orientation::LeftUp),
        _ => None,
    }
}

/// The orientation that the two buttons of 90 degrees start from: the
/// orientation of the panels if it is left side up, upright or right side up,
/// else upright, so that both buttons stay usable (also after a screen was
/// turned upside down by hand).
#[must_use]
pub fn buttons_from(orientation: Option<Orientation>) -> Orientation {
    orientation
        .filter(|orientation| {
            matches!(
                orientation,
                Orientation::LeftUp | Orientation::Normal | Orientation::RightUp
            )
        })
        .unwrap_or(Orientation::Normal)
}

/// The orientation that the built-in panels are turned to. `None` if no panel
/// is shown by itself, or the panels are turned differently.
#[must_use]
pub fn orientation_of(state: &State) -> Option<Orientation> {
    let panels = panels_of(state);
    let transform = state.logical[*panels.first()?].transform;

    if panels
        .iter()
        .any(|&at| state.logical[at].transform != transform)
    {
        return None;
    }
    Orientation::from_transform(transform)
}
