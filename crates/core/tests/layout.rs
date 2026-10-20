use tesserae_core::layout::{Direction, Divider, PaneId, PaneLayout, Rect, SplitDir, SplitTree};

fn id(n: u64) -> PaneId {
    PaneId(n)
}

#[test]
fn single_leaf_fills_area() {
    let tree = SplitTree::new(id(1));
    let area = Rect::new(0, 0, 80, 24);
    let layout = tree.layout(area);

    assert_eq!(layout.panes.len(), 1);
    assert_eq!(layout.panes[0].id, id(1));
    assert_eq!(layout.panes[0].rect, area);
    assert!(layout.dividers.is_empty());
}

#[test]
fn columns_split_creates_divider() {
    let mut tree = SplitTree::new(id(1));
    assert!(tree.split(id(1), id(2), SplitDir::Columns));

    let layout = tree.layout(Rect::new(0, 0, 80, 24));

    assert_eq!(
        layout.panes,
        vec![
            PaneLayout {
                id: id(1),
                rect: Rect::new(0, 0, 39, 24),
            },
            PaneLayout {
                id: id(2),
                rect: Rect::new(40, 0, 40, 24),
            },
        ]
    );
    assert_eq!(
        layout.dividers,
        vec![Divider {
            rect: Rect::new(39, 0, 1, 24),
            dir: SplitDir::Columns,
        }]
    );
}

#[test]
fn rows_split_creates_divider() {
    let mut tree = SplitTree::new(id(1));
    assert!(tree.split(id(1), id(2), SplitDir::Rows));

    let layout = tree.layout(Rect::new(0, 0, 80, 24));

    assert_eq!(
        layout.panes,
        vec![
            PaneLayout {
                id: id(1),
                rect: Rect::new(0, 0, 80, 11),
            },
            PaneLayout {
                id: id(2),
                rect: Rect::new(0, 12, 80, 12),
            },
        ]
    );
    assert_eq!(
        layout.dividers,
        vec![Divider {
            rect: Rect::new(0, 11, 80, 1),
            dir: SplitDir::Rows,
        }]
    );
}

#[test]
fn split_keeps_existing_pane_first() {
    let mut tree = SplitTree::new(id(1));
    assert!(tree.split(id(1), id(2), SplitDir::Columns));

    assert_eq!(tree.leaves(), vec![id(1), id(2)]);
}

#[test]
fn splits_missing_pane_returns_false() {
    let mut tree = SplitTree::new(id(1));

    assert!(!tree.split(id(9), id(2), SplitDir::Columns));
    assert_eq!(tree.leaves(), vec![id(1)]);
}

#[test]
fn nested_splits_layout_in_order() {
    let mut tree = SplitTree::new(id(1));
    assert!(tree.split(id(1), id(2), SplitDir::Columns));
    assert!(tree.split(id(2), id(3), SplitDir::Rows));

    let layout = tree.layout(Rect::new(0, 0, 80, 24));

    assert_eq!(
        layout.panes,
        vec![
            PaneLayout {
                id: id(1),
                rect: Rect::new(0, 0, 39, 24),
            },
            PaneLayout {
                id: id(2),
                rect: Rect::new(40, 0, 40, 11),
            },
            PaneLayout {
                id: id(3),
                rect: Rect::new(40, 12, 40, 12),
            },
        ]
    );
    assert_eq!(
        layout.dividers,
        vec![
            Divider {
                rect: Rect::new(39, 0, 1, 24),
                dir: SplitDir::Columns,
            },
            Divider {
                rect: Rect::new(40, 11, 40, 1),
                dir: SplitDir::Rows,
            },
        ]
    );
}

#[test]
fn remove_collapses_parent_and_promotes_sibling() {
    let mut tree = SplitTree::new(id(1));
    assert!(tree.split(id(1), id(2), SplitDir::Columns));
    assert!(tree.split(id(2), id(3), SplitDir::Rows));

    assert!(tree.remove(id(2)));
    assert_eq!(tree.leaves(), vec![id(1), id(3)]);

    assert!(tree.remove(id(1)));
    assert_eq!(tree.leaves(), vec![id(3)]);

    assert!(tree.remove(id(3)));
    assert!(tree.is_empty());
    assert!(!tree.remove(id(3)));
}

#[test]
fn pane_at_hit_tests_panes_dividers_and_bounds() {
    let mut tree = SplitTree::new(id(1));
    assert!(tree.split(id(1), id(2), SplitDir::Columns));
    let layout = tree.layout(Rect::new(0, 0, 80, 24));

    assert_eq!(layout.pane_at(0, 0), Some(id(1)));
    assert_eq!(layout.pane_at(39, 0), None);
    assert_eq!(layout.pane_at(40, 0), Some(id(2)));
    assert_eq!(layout.pane_at(79, 23), Some(id(2)));
    assert_eq!(layout.pane_at(80, 0), None);
}

#[test]
fn directional_focus_columns() {
    let mut tree = SplitTree::new(id(1));
    assert!(tree.split(id(1), id(2), SplitDir::Columns));
    let layout = tree.layout(Rect::new(0, 0, 80, 24));

    assert_eq!(
        layout.pane_in_direction(id(1), Direction::Right),
        Some(id(2))
    );
    assert_eq!(
        layout.pane_in_direction(id(2), Direction::Left),
        Some(id(1))
    );
    assert_eq!(layout.pane_in_direction(id(1), Direction::Up), None);
}

#[test]
fn directional_focus_rows() {
    let mut tree = SplitTree::new(id(1));
    assert!(tree.split(id(1), id(2), SplitDir::Rows));
    let layout = tree.layout(Rect::new(0, 0, 80, 24));

    assert_eq!(
        layout.pane_in_direction(id(1), Direction::Down),
        Some(id(2))
    );
    assert_eq!(layout.pane_in_direction(id(2), Direction::Up), Some(id(1)));
}

#[test]
fn split_with_ratio() {
    let mut tree = SplitTree::new(id(1));
    assert!(tree.split_with_ratio(id(1), id(2), SplitDir::Columns, 25));

    let layout = tree.layout(Rect::new(0, 0, 80, 24));

    assert_eq!(layout.panes[0].rect.width, 19);
    assert_eq!(layout.panes[1].rect.width, 60);
    assert_eq!(layout.dividers[0].rect.x, 19);
    assert_eq!(layout.dividers[0].rect.width, 1);
}

#[test]
fn tiny_areas_do_not_panic() {
    let mut tree = SplitTree::new(id(1));
    assert!(tree.split(id(1), id(2), SplitDir::Columns));

    let layout = tree.layout(Rect::new(0, 0, 1, 1));

    assert_eq!(layout.panes.len(), 2);
    assert_eq!(layout.panes[0].rect, Rect::new(0, 0, 1, 1));
    assert_eq!(layout.panes[1].rect, Rect::new(1, 0, 0, 1));
    assert!(layout.dividers[0].rect.is_empty());
    assert_eq!(layout.pane_at(0, 0), Some(id(1)));
    assert_eq!(layout.pane_at(1, 0), None);
}
