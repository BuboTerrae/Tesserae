use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    layout::{Direction, Layout, PaneId, Rect, SplitDir, SplitTree},
    terminal::{Snapshot, TerminalSession},
};

struct Pane {
    session: TerminalSession,
    cwd: Option<PathBuf>,
}

pub struct Workspace {
    shell: String,
    tree: SplitTree,
    panes: HashMap<PaneId, Pane>,
    focused: PaneId,
    next_id: u64,
    area: Rect,
}

impl Workspace {
    pub fn new(shell: &str, area: Rect) -> Result<Self, String> {
        let cwd = None;
        let area = normalize_area(area);
        let first = PaneId(1);
        let session = TerminalSession::spawn_in(shell, area.width, area.height, cwd)?;

        let mut panes = HashMap::new();
        let _ = panes.insert(
            first,
            Pane {
                session,
                cwd: cwd.map(Path::to_path_buf),
            },
        );

        Ok(Self {
            shell: shell.to_string(),
            tree: SplitTree::new(first),
            panes,
            focused: first,
            next_id: 2,
            area,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.panes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.panes.len()
    }

    pub fn area(&self) -> Rect {
        self.area
    }

    pub fn focused(&self) -> PaneId {
        self.focused
    }

    pub fn focus(&mut self, id: PaneId) -> bool {
        if self.panes.contains_key(&id) && self.tree.contains(id) {
            self.focused = id;
            true
        } else {
            false
        }
    }

    pub fn focus_next(&mut self) -> bool {
        let leaves = self.tree.leaves();
        if leaves.len() < 2 {
            return false;
        }

        let index = leaves
            .iter()
            .position(|id| *id == self.focused)
            .unwrap_or(0);
        self.focused = leaves[(index + 1) % leaves.len()];
        true
    }

    pub fn focus_direction(&mut self, dir: Direction) -> bool {
        let Some(id) = self.layout().pane_in_direction(self.focused, dir) else {
            return false;
        };
        self.focus(id)
    }

    pub fn focus_at(&mut self, x: u16, y: u16) -> bool {
        let Some(id) = self.layout().pane_at(x, y) else {
            return false;
        };
        self.focus(id)
    }

    pub fn layout(&self) -> Layout {
        self.tree.layout(self.area)
    }

    pub fn pane_rect(&self, id: PaneId) -> Option<Rect> {
        self.layout().pane(id).map(|pane| pane.rect)
    }

    pub fn pane_cwd(&self, id: PaneId) -> Option<PathBuf> {
        let pane = self.panes.get(&id)?;

        live_cwd(pane.session.process_id())
            .filter(|path| path.is_dir())
            .or_else(|| pane.session.reported_cwd().filter(|path| path.is_dir()))
            .or_else(|| pane.cwd.clone().filter(|path| path.is_dir()))
    }

    pub fn split_focused(&mut self, dir: SplitDir) -> Result<bool, String> {
        let parent = self.focused;
        let Some(parent_rect) = self.pane_rect(parent) else {
            return Ok(false);
        };

        let available = match dir {
            SplitDir::Columns => parent_rect.width,
            SplitDir::Rows => parent_rect.height,
        };
        if available < 3 {
            return Ok(false);
        }

        let id = PaneId(self.next_id);
        let cwd = self.pane_cwd(parent);
        let session = TerminalSession::spawn_in(
            &self.shell,
            parent_rect.width.max(1),
            parent_rect.height.max(1),
            cwd.as_deref(),
        )?;

        if !self.tree.split(parent, id, dir) {
            return Ok(false);
        }

        let _ = self.panes.insert(
            id,
            Pane {
                session,
                cwd: cwd.clone(),
            },
        );
        self.next_id += 1;
        self.focused = id;

        if let Err(err) = self.resize(self.area) {
            self.tree.remove(id);
            let _ = self.panes.remove(&id);
            self.focused = parent;
            return Err(err);
        }

        Ok(true)
    }

    pub fn close(&mut self, id: PaneId) -> bool {
        let leaves = self.tree.leaves();
        let Some(index) = leaves.iter().position(|leaf| *leaf == id) else {
            return false;
        };

        if !self.tree.remove(id) {
            return false;
        }
        let _ = self.panes.remove(&id);

        if self.focused == id && !self.is_empty() {
            let remaining = self.tree.leaves();
            if !remaining.is_empty() {
                self.focused = remaining[index % remaining.len()];
            }
        }

        true
    }

    pub fn close_focused(&mut self) -> bool {
        self.close(self.focused)
    }

    pub fn resize(&mut self, area: Rect) -> Result<(), String> {
        self.area = normalize_area(area);
        let layout = self.tree.layout(self.area);

        for pane_layout in layout.panes {
            if let Some(pane) = self.panes.get(&pane_layout.id) {
                pane.session.resize(
                    pane_layout.rect.width.max(1),
                    pane_layout.rect.height.max(1),
                )?;
            }
        }

        Ok(())
    }

    pub fn poll_exits(&mut self) -> Result<bool, String> {
        let mut exited = Vec::new();

        for (id, pane) in self.panes.iter_mut() {
            if pane.session.child_exited()? {
                exited.push(*id);
            }
        }

        for id in exited {
            let _ = self.close(id);
        }

        Ok(self.is_empty())
    }

    pub fn write_focused(&self, bytes: &[u8]) -> Result<(), String> {
        match self.panes.get(&self.focused) {
            Some(pane) => pane.session.write(bytes),
            None => Err("no focused pane".to_string()),
        }
    }

    pub fn scroll(&self, id: PaneId, delta: i32) -> bool {
        match self.panes.get(&id) {
            Some(pane) => {
                pane.session.scroll(delta);
                true
            }
            None => false,
        }
    }

    pub fn scroll_focused(&self, delta: i32) {
        if let Some(pane) = self.panes.get(&self.focused) {
            pane.session.scroll(delta);
        }
    }

    pub fn snapshot(&self, id: PaneId) -> Option<Snapshot> {
        self.panes.get(&id).map(|pane| pane.session.snapshot())
    }

    pub fn snapshot_focused(&self) -> Option<Snapshot> {
        self.snapshot(self.focused)
    }

    pub fn snapshots(&self) -> Vec<(PaneId, Rect, Snapshot)> {
        let layout = self.tree.layout(self.area);

        layout
            .panes
            .into_iter()
            .filter_map(|pane_layout| {
                self.panes
                    .get(&pane_layout.id)
                    .map(|pane| (pane_layout.id, pane_layout.rect, pane.session.snapshot()))
            })
            .collect()
    }
}

fn normalize_area(area: Rect) -> Rect {
    Rect {
        width: area.width.max(1),
        height: area.height.max(1),
        ..area
    }
}

#[cfg(target_os = "linux")]
fn live_cwd(pid: Option<u32>) -> Option<PathBuf> {
    let pid = pid?;
    std::fs::read_link(format!("/proc/{pid}/cwd")).ok()
}

#[cfg(not(target_os = "linux"))]
fn live_cwd(_pid: Option<u32>) -> Option<PathBuf> {
    None
}
