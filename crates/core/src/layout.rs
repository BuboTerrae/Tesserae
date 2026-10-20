#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PaneId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitDir {
    Columns,
    Rows,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> u16 {
        self.x.saturating_add(self.width)
    }

    pub fn bottom(&self) -> u16 {
        self.y.saturating_add(self.height)
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn contains(&self, x: u16, y: u16) -> bool {
        !self.is_empty() && x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    pub fn center(&self) -> (u16, u16) {
        (
            self.x.saturating_add(self.width / 2),
            self.y.saturating_add(self.height / 2),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Leaf(PaneId),
    Split {
        dir: SplitDir,
        ratio: u16,
        first: Box<Node>,
        second: Box<Node>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitTree {
    root: Option<Node>,
}

impl SplitTree {
    pub fn new(root: PaneId) -> Self {
        Self {
            root: Some(Node::Leaf(root)),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    pub fn contains(&self, id: PaneId) -> bool {
        self.root
            .as_ref()
            .is_some_and(|node| node_contains(node, id))
    }

    pub fn leaves(&self) -> Vec<PaneId> {
        let mut leaves = Vec::new();
        if let Some(root) = &self.root {
            collect_leaves(root, &mut leaves);
        }
        leaves
    }

    pub fn split(&mut self, target: PaneId, new: PaneId, dir: SplitDir) -> bool {
        self.split_with_ratio(target, new, dir, 50)
    }

    pub fn split_with_ratio(
        &mut self,
        target: PaneId,
        new: PaneId,
        dir: SplitDir,
        ratio: u16,
    ) -> bool {
        let ratio = ratio.clamp(1, 99);
        match self.root.as_mut() {
            Some(node) => split_node(node, target, new, dir, ratio),
            None => false,
        }
    }

    pub fn remove(&mut self, target: PaneId) -> bool {
        let Some(root) = self.root.take() else {
            return false;
        };
        let (root, removed) = remove_node(root, target);
        self.root = root;
        removed
    }

    pub fn layout(&self, area: Rect) -> Layout {
        let mut layout = Layout::default();
        if let Some(root) = &self.root {
            visit(root, area, &mut layout);
        }
        layout
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaneLayout {
    pub id: PaneId,
    pub rect: Rect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Divider {
    pub rect: Rect,
    pub dir: SplitDir,
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Layout {
    pub panes: Vec<PaneLayout>,
    pub dividers: Vec<Divider>,
}

impl Layout {
    pub fn pane(&self, id: PaneId) -> Option<&PaneLayout> {
        self.panes.iter().find(|pane| pane.id == id)
    }

    pub fn pane_at(&self, x: u16, y: u16) -> Option<PaneId> {
        self.panes
            .iter()
            .find(|pane| pane.rect.contains(x, y))
            .map(|pane| pane.id)
    }

    pub fn pane_in_direction(&self, from: PaneId, dir: Direction) -> Option<PaneId> {
        let from_rect = self.pane(from)?.rect;
        let (from_x, from_y) = from_rect.center();
        let mut best: Option<(PaneId, u32)> = None;

        for pane in &self.panes {
            if pane.id == from || pane.rect.is_empty() {
                continue;
            }

            let (x, y) = pane.rect.center();
            let dx = x as i32 - from_x as i32;
            let dy = y as i32 - from_y as i32;

            let (primary, secondary) = match dir {
                Direction::Left => (-dx, dy.abs()),
                Direction::Right => (dx, dy.abs()),
                Direction::Up => (-dy, dx.abs()),
                Direction::Down => (dy, dx.abs()),
            };

            if primary <= 0 {
                continue;
            }

            let score = primary as u32 * 2 + secondary as u32;
            let better = match best {
                None => true,
                Some((_, best_score)) => score < best_score,
            };

            if better {
                best = Some((pane.id, score));
            }
        }

        best.map(|(id, _)| id)
    }
}

fn node_contains(node: &Node, id: PaneId) -> bool {
    match node {
        Node::Leaf(pane) => *pane == id,
        Node::Split { first, second, .. } => node_contains(first, id) || node_contains(second, id),
    }
}

fn collect_leaves(node: &Node, leaves: &mut Vec<PaneId>) {
    match node {
        Node::Leaf(id) => leaves.push(*id),
        Node::Split { first, second, .. } => {
            collect_leaves(first, leaves);
            collect_leaves(second, leaves);
        }
    }
}

fn split_node(node: &mut Node, target: PaneId, new: PaneId, dir: SplitDir, ratio: u16) -> bool {
    match node {
        Node::Leaf(id) if *id == target => {
            let existing = *id;
            *node = Node::Split {
                dir,
                ratio,
                first: Box::new(Node::Leaf(existing)),
                second: Box::new(Node::Leaf(new)),
            };
            true
        }
        Node::Leaf(_) => false,
        Node::Split { first, second, .. } => {
            split_node(first, target, new, dir, ratio)
                || split_node(second, target, new, dir, ratio)
        }
    }
}

fn remove_node(node: Node, target: PaneId) -> (Option<Node>, bool) {
    match node {
        Node::Leaf(id) if id == target => (None, true),
        Node::Leaf(id) => (Some(Node::Leaf(id)), false),
        Node::Split {
            dir,
            ratio,
            first,
            second,
        } => {
            let (first, removed) = remove_node(*first, target);
            if removed {
                return match first {
                    Some(first) => (
                        Some(Node::Split {
                            dir,
                            ratio,
                            first: Box::new(first),
                            second,
                        }),
                        true,
                    ),
                    None => (Some(*second), true),
                };
            }

            let first = first.expect("unremoved subtree");
            let (second, removed) = remove_node(*second, target);
            if removed {
                return match second {
                    Some(second) => (
                        Some(Node::Split {
                            dir,
                            ratio,
                            first: Box::new(first),
                            second: Box::new(second),
                        }),
                        true,
                    ),
                    None => (Some(first), true),
                };
            }

            let second = second.expect("unremoved subtree");
            (
                Some(Node::Split {
                    dir,
                    ratio,
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                false,
            )
        }
    }
}

fn visit(node: &Node, area: Rect, layout: &mut Layout) {
    match node {
        Node::Leaf(id) => layout.panes.push(PaneLayout {
            id: *id,
            rect: area,
        }),
        Node::Split {
            dir,
            ratio,
            first,
            second,
        } => {
            let (first_rect, divider, second_rect) = split_rect(area, *dir, *ratio);
            layout.dividers.push(Divider {
                rect: divider,
                dir: *dir,
            });
            visit(first, first_rect, layout);
            visit(second, second_rect, layout);
        }
    }
}

fn split_rect(area: Rect, dir: SplitDir, ratio: u16) -> (Rect, Rect, Rect) {
    let ratio = ratio.clamp(1, 99) as u32;

    match dir {
        SplitDir::Columns => {
            if area.width < 3 {
                let divider = Rect::new(area.right(), area.y, 0, area.height);
                let second = Rect::new(area.right(), area.y, 0, area.height);
                return (area, divider, second);
            }

            let space = area.width - 1;
            let first = ((space as u32 * ratio) / 100) as u16;
            let first = first.clamp(1, space - 1);
            let divider = Rect::new(area.x + first, area.y, 1, area.height);
            let second = Rect::new(area.x + first + 1, area.y, space - first, area.height);
            (
                Rect::new(area.x, area.y, first, area.height),
                divider,
                second,
            )
        }
        SplitDir::Rows => {
            if area.height < 3 {
                let divider = Rect::new(area.x, area.bottom(), area.width, 0);
                let second = Rect::new(area.x, area.bottom(), area.width, 0);
                return (area, divider, second);
            }

            let space = area.height - 1;
            let first = ((space as u32 * ratio) / 100) as u16;
            let first = first.clamp(1, space - 1);
            let divider = Rect::new(area.x, area.y + first, area.width, 1);
            let second = Rect::new(area.x, area.y + first + 1, area.width, space - first);
            (
                Rect::new(area.x, area.y, area.width, first),
                divider,
                second,
            )
        }
    }
}
