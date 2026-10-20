#![cfg(unix)]

use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};

use tesserae_core::{
    layout::{Direction, PaneId, Rect, SplitDir},
    workspace::Workspace,
};

fn area() -> Rect {
    Rect::new(0, 0, 80, 24)
}

fn contains(workspace: &Workspace, id: PaneId, needle: &str) -> bool {
    let Some(snapshot) = workspace.snapshot(id) else {
        return false;
    };

    (0..snapshot.rows).any(|y| {
        snapshot
            .line(y)
            .iter()
            .map(|cell| cell.ch)
            .collect::<String>()
            .contains(needle)
    })
}

fn wait_for(workspace: &Workspace, id: PaneId, needle: &str) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if contains(workspace, id, needle) {
            return true;
        }
        thread::sleep(Duration::from_millis(20));
    }
    false
}

fn wait_for_cwd(workspace: &Workspace, id: PaneId, cwd: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if workspace.pane_cwd(id).as_deref() == Some(cwd) {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }

    panic!("timed out waiting for cwd {:?}", cwd);
}

#[test]
fn new_workspace_has_one_pane() {
    let workspace = Workspace::new("/bin/sh", area()).unwrap();

    assert_eq!(workspace.len(), 1);
    assert!(!workspace.is_empty());

    let layout = workspace.layout();
    assert_eq!(layout.panes.len(), 1);
    assert_eq!(layout.panes[0].rect, area());
    assert_eq!(workspace.focused(), PaneId(1));
}

#[test]
fn columns_split_focuses_new_pane() {
    let mut workspace = Workspace::new("/bin/sh", area()).unwrap();
    assert!(workspace.split_focused(SplitDir::Columns).unwrap());

    let layout = workspace.layout();
    assert_eq!(workspace.len(), 2);
    assert_eq!(layout.panes[0].rect, Rect::new(0, 0, 39, 24));
    assert_eq!(layout.panes[1].rect, Rect::new(40, 0, 40, 24));
    assert_eq!(workspace.focused(), layout.panes[1].id);

    let snapshots = workspace.snapshots();
    assert_eq!(snapshots.len(), 2);
    assert_eq!(snapshots[1].1, Rect::new(40, 0, 40, 24));
}

#[test]
fn rows_split_focuses_new_pane() {
    let mut workspace = Workspace::new("/bin/sh", area()).unwrap();
    assert!(workspace.split_focused(SplitDir::Rows).unwrap());

    let layout = workspace.layout();
    assert_eq!(layout.panes[0].rect, Rect::new(0, 0, 80, 11));
    assert_eq!(layout.panes[1].rect, Rect::new(0, 12, 80, 12));
    assert_eq!(workspace.focused(), layout.panes[1].id);
}

#[test]
fn split_refuses_tiny_pane() {
    let mut workspace = Workspace::new("/bin/sh", Rect::new(0, 0, 2, 24)).unwrap();

    assert!(!workspace.split_focused(SplitDir::Columns).unwrap());
    assert_eq!(workspace.len(), 1);
}

#[test]
fn write_goes_to_focused_pane() {
    let mut workspace = Workspace::new("/bin/sh", area()).unwrap();
    let first = workspace.focused();

    assert!(workspace.split_focused(SplitDir::Columns).unwrap());
    let second = workspace.focused();

    workspace
        .write_focused(b"echo only-in-second-pane\n")
        .unwrap();

    assert!(wait_for(&workspace, second, "only-in-second-pane"));
    assert!(!contains(&workspace, first, "only-in-second-pane"));
}

#[test]
fn close_focused_promotes_sibling() {
    let mut workspace = Workspace::new("/bin/sh", area()).unwrap();
    let first = workspace.focused();

    assert!(workspace.split_focused(SplitDir::Columns).unwrap());
    let second = workspace.focused();

    assert!(workspace.close_focused());
    assert_eq!(workspace.len(), 1);
    assert_eq!(workspace.focused(), first);
    assert!(workspace.snapshot(second).is_none());
}

#[test]
fn close_last_pane_empties_workspace() {
    let mut workspace = Workspace::new("/bin/sh", area()).unwrap();

    assert!(workspace.close_focused());
    assert!(workspace.is_empty());
    assert!(workspace.layout().panes.is_empty());
    assert!(workspace.snapshot_focused().is_none());
}

#[test]
fn poll_exits_removes_dead_pane() {
    let mut workspace = Workspace::new("/bin/sh", area()).unwrap();
    workspace.write_focused(b"exit\n").unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if workspace.poll_exits().unwrap() {
            assert!(workspace.is_empty());
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }

    panic!("timed out waiting for pane exit");
}

#[test]
fn poll_exits_keeps_live_sibling() {
    let mut workspace = Workspace::new("/bin/sh", area()).unwrap();
    let first = workspace.focused();

    assert!(workspace.split_focused(SplitDir::Columns).unwrap());
    let second = workspace.focused();

    assert!(workspace.focus(first));
    workspace.write_focused(b"exit\n").unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let empty = workspace.poll_exits().unwrap();
        assert!(!empty, "live sibling should prevent an empty workspace");

        if workspace.len() == 1 {
            assert_eq!(workspace.focused(), second);
            return;
        }

        thread::sleep(Duration::from_millis(20));
    }

    panic!("timed out waiting for pane exit");
}

#[test]
fn resize_updates_all_panes() {
    let mut workspace = Workspace::new("/bin/sh", area()).unwrap();
    let first = workspace.focused();

    assert!(workspace.split_focused(SplitDir::Columns).unwrap());
    let second = workspace.focused();

    workspace.resize(Rect::new(0, 0, 100, 30)).unwrap();

    let first_snapshot = workspace.snapshot(first).unwrap();
    let second_snapshot = workspace.snapshot(second).unwrap();
    assert_eq!(first_snapshot.cols, 49);
    assert_eq!(first_snapshot.rows, 30);
    assert_eq!(second_snapshot.cols, 50);
    assert_eq!(second_snapshot.rows, 30);
}

#[test]
fn focus_direction_and_click() {
    let mut workspace = Workspace::new("/bin/sh", area()).unwrap();
    let first = workspace.focused();

    assert!(workspace.split_focused(SplitDir::Columns).unwrap());
    let second = workspace.focused();

    assert!(workspace.focus(first));
    assert!(workspace.focus_direction(Direction::Right));
    assert_eq!(workspace.focused(), second);

    assert!(workspace.focus(first));
    assert!(workspace.focus_at(0, 0));
    assert_eq!(workspace.focused(), first);

    assert!(workspace.focus_at(79, 23));
    assert_eq!(workspace.focused(), second);
}

#[cfg(target_os = "linux")]
#[test]
fn pane_cwd_tracks_shell_directory() {
    let mut workspace = Workspace::new("/bin/sh", area()).unwrap();
    let first = workspace.focused();

    assert!(workspace.pane_cwd(first).is_some());
    workspace.write_focused(b"cd /tmp\n").unwrap();
    wait_for_cwd(&workspace, first, Path::new("/tmp"));
}

#[cfg(target_os = "linux")]
#[test]
fn split_inherits_focused_cwd() {
    let mut workspace = Workspace::new("/bin/sh", area()).unwrap();
    let first = workspace.focused();

    workspace.write_focused(b"cd /tmp\n").unwrap();
    wait_for_cwd(&workspace, first, Path::new("/tmp"));

    assert!(workspace.split_focused(SplitDir::Columns).unwrap());
    let second = workspace.focused();
    wait_for_cwd(&workspace, second, Path::new("/tmp"));

    workspace.write_focused(b"pwd\n").unwrap();
    assert!(wait_for(&workspace, second, "/tmp"));
}
