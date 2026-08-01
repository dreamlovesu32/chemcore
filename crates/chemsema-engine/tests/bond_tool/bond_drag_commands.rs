use super::*;

#[test]
fn alt_drag_from_endpoint_uses_mouse_distance_without_snap() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: true,
    });
    engine.pointer_move(PointerEvent {
        x: px(389.0),
        y: px(301.0),
        button: None,
        alt_key: true,
    });
    let preview = engine.state().overlay.preview.as_ref().unwrap();
    assert!((preview.end.x - px(389.0)).abs() < 0.001, "{preview:?}");
    assert!((preview.end.y - px(301.0)).abs() < 0.001, "{preview:?}");
    engine.pointer_up(PointerEvent {
        x: px(389.0),
        y: px(301.0),
        button: Some(0),
        alt_key: true,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    assert!(
        (last.position[0] - px(389.0)).abs() < 0.001,
        "{:?}",
        last.position
    );
    assert!(
        (last.position[1] - px(301.0)).abs() < 0.001,
        "{:?}",
        last.position
    );
    let length = ((last.position[0] - FIRST_END_X).powi(2)
        + (last.position[1] - FIRST_END_Y).powi(2))
    .sqrt();
    assert!((length - DEFAULT_BOND_LENGTH).abs() > px(5.0), "{length}");
}

#[test]
fn click_on_blank_canvas_creates_up_right_triple_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(triple_bond_tool());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 2);
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(entry.fragment.nodes[1].position, [FIRST_END_X, FIRST_END_Y]);
    assert_eq!(entry.fragment.bonds[0].order, 3);
}

#[test]
fn click_on_blank_canvas_creates_up_right_double_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(double_bond_tool());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 2);
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(entry.fragment.nodes[1].position, [FIRST_END_X, FIRST_END_Y]);
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 2);
    assert_eq!(
        bond.double.as_ref().map(|double| double.frozen),
        Some(false)
    );
}

#[test]
fn click_on_triple_bond_endpoint_extends_at_180_degrees() {
    let mut engine = Engine::new();
    engine.set_tool_state(triple_bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 3);
    let point = entry.fragment.nodes[2].position;
    assert!(
        (point[0] - FIRST_END_TRIPLE_EXTEND_X).abs() < 0.01,
        "{point:?}"
    );
    assert!(
        (point[1] - FIRST_END_TRIPLE_EXTEND_Y).abs() < 0.01,
        "{point:?}"
    );
    assert_eq!(entry.fragment.bonds[1].order, 3);
}

#[test]
fn click_on_blank_canvas_creates_up_right_dashed_single_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(dashed_bond_tool());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes[1].position, [FIRST_END_X, FIRST_END_Y]);
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(entry.fragment.bonds[0].order, 1);
    assert_eq!(
        entry.fragment.bonds[0].line_styles.main,
        BondLinePattern::Dashed
    );
}

#[test]
fn click_on_blank_canvas_creates_up_right_dashed_double_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(dashed_double_bond_tool());

    click(&mut engine, px(300.0), px(260.0));

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes[1].position, [FIRST_END_X, FIRST_END_Y]);
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 2);
    assert!(matches!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right | DoubleBondPlacement::Center)
    ));
    assert_eq!(
        bond.double.as_ref().map(|double| double.frozen),
        Some(false)
    );
    assert_eq!(bond.line_styles.main, BondLinePattern::Solid);
    assert_eq!(bond.line_styles.right, BondLinePattern::Dashed);
}

#[test]
fn dashed_double_tool_cycles_side_center_and_opposite_side() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(dashed_double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let active_side = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        let side = bond
            .double
            .as_ref()
            .map(|double| double.placement)
            .expect("side dashed-double placement");
        assert!(matches!(
            side,
            DoubleBondPlacement::Left | DoubleBondPlacement::Right
        ));
        assert_eq!(bond.order, 2);
        assert_eq!(bond.double.as_ref().map(|double| double.frozen), Some(true));
        assert_eq!(
            match side {
                DoubleBondPlacement::Left => bond.line_styles.left,
                DoubleBondPlacement::Right => bond.line_styles.right,
                DoubleBondPlacement::Center => unreachable!(),
            },
            BondLinePattern::Dashed
        );
        assert_eq!(
            match side {
                DoubleBondPlacement::Left => bond.line_styles.right,
                DoubleBondPlacement::Right => bond.line_styles.left,
                DoubleBondPlacement::Center => unreachable!(),
            },
            BondLinePattern::Solid
        );
        side
    };

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    {
        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        assert_eq!(
            bond.double.as_ref().map(|double| double.placement),
            Some(DoubleBondPlacement::Center)
        );
        assert_eq!(
            match active_side {
                DoubleBondPlacement::Left => bond.line_styles.left,
                DoubleBondPlacement::Right => bond.line_styles.right,
                DoubleBondPlacement::Center => unreachable!(),
            },
            BondLinePattern::Dashed
        );
        assert_eq!(
            match active_side {
                DoubleBondPlacement::Left => bond.line_styles.right,
                DoubleBondPlacement::Right => bond.line_styles.left,
                DoubleBondPlacement::Center => unreachable!(),
            },
            BondLinePattern::Solid
        );
    }

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let opposite_side = match active_side {
        DoubleBondPlacement::Left => DoubleBondPlacement::Right,
        DoubleBondPlacement::Right => DoubleBondPlacement::Left,
        DoubleBondPlacement::Center => unreachable!(),
    };
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(opposite_side)
    );
    assert_eq!(
        match opposite_side {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Dashed
    );
    assert_eq!(
        match opposite_side {
            DoubleBondPlacement::Left => bond.line_styles.right,
            DoubleBondPlacement::Right => bond.line_styles.left,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Solid
    );
}

#[test]
fn dashed_tool_click_on_single_bond_makes_it_dashed() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(
        entry.fragment.bonds[0].line_styles.main,
        BondLinePattern::Dashed
    );
    assert_eq!(entry.fragment.bonds[0].stroke_width, DEFAULT_BOND_STROKE);
}

#[test]
fn dashed_tool_resets_non_double_styles_to_plain_dashed_single() {
    let mut engine = Engine::new();
    engine.set_tool_state(bold_bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 1);
    assert!(bond.double.is_none());
    assert!(bond.stereo.is_none());
    assert_eq!(bond.line_styles.main, BondLinePattern::Dashed);
    assert_eq!(bond.line_styles.left, BondLinePattern::Solid);
    assert_eq!(bond.line_weights.main, BondLineWeight::Normal);
}

#[test]
fn dashed_tool_cycles_side_double_states() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let active_side = bond
        .double
        .as_ref()
        .map(|double| double.placement)
        .expect("side double placement");
    let opposite_side = match active_side {
        DoubleBondPlacement::Left => DoubleBondPlacement::Right,
        DoubleBondPlacement::Right => DoubleBondPlacement::Left,
        DoubleBondPlacement::Center => unreachable!("side double should not be centered"),
    };
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(active_side)
    );
    assert_eq!(bond.line_styles.main, BondLinePattern::Solid);
    assert_eq!(
        match active_side {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Dashed
    );

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(active_side)
    );
    assert_eq!(bond.line_styles.main, BondLinePattern::Dashed);
    assert_eq!(
        match active_side {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Dashed
    );

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let double = bond.double.as_ref().unwrap();
    assert_eq!(double.placement, DoubleBondPlacement::Center);
    assert_eq!(double.center_exit_side, Some(opposite_side));
    assert_eq!(bond.line_styles.left, BondLinePattern::Dashed);
    assert_eq!(bond.line_styles.right, BondLinePattern::Dashed);

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(opposite_side)
    );
    assert_eq!(bond.line_styles.main, BondLinePattern::Solid);
    assert_eq!(
        match opposite_side {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Dashed
    );
    assert_eq!(
        match active_side {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Solid
    );
}

#[test]
fn dashed_tool_cycles_center_double_states() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert!(matches!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right | DoubleBondPlacement::Center)
    ));
    let first_dashed = if bond.line_styles.left == BondLinePattern::Dashed {
        DoubleBondPlacement::Left
    } else {
        DoubleBondPlacement::Right
    };
    let second_dashed = match first_dashed {
        DoubleBondPlacement::Left => DoubleBondPlacement::Right,
        DoubleBondPlacement::Right => DoubleBondPlacement::Left,
        DoubleBondPlacement::Center => unreachable!(),
    };
    assert_eq!(
        match first_dashed {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Dashed
    );
    assert_eq!(
        match second_dashed {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Solid
    );

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let double = bond.double.as_ref().unwrap();
    assert_eq!(double.placement, DoubleBondPlacement::Center);
    assert_eq!(double.center_exit_side, Some(second_dashed));
    assert_eq!(bond.line_styles.left, BondLinePattern::Dashed);
    assert_eq!(bond.line_styles.right, BondLinePattern::Dashed);

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(second_dashed)
    );
    assert_eq!(bond.line_styles.main, BondLinePattern::Solid);
    assert_eq!(
        match second_dashed {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Dashed
    );
    assert_eq!(
        match first_dashed {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Solid
    );
}

#[test]
fn click_on_blank_canvas_creates_horizontal_bold_single_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bold_bond_tool());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(entry.fragment.bonds[0].order, 1);
    assert_eq!(
        entry.fragment.bonds[0].line_weights.main,
        BondLineWeight::Bold
    );
    assert_eq!(entry.fragment.bonds[0].stroke_width, DEFAULT_BOND_STROKE);
}

#[test]
fn bold_tool_click_on_single_bond_makes_it_bold() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(bold_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.bonds[0].order, 1);
    assert_eq!(
        entry.fragment.bonds[0].line_weights.main,
        BondLineWeight::Bold
    );
    assert_eq!(entry.fragment.bonds[0].stroke_width, DEFAULT_BOND_STROKE);
}

#[test]
fn bold_tool_cycles_single_and_side_double_states() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(bold_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 1);
    assert_eq!(bond.line_weights.main, BondLineWeight::Bold);

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let active_side = bond
        .double
        .as_ref()
        .map(|double| double.placement)
        .expect("side double placement");
    assert_eq!(bond.order, 2);
    assert_eq!(bond.line_weights.main, BondLineWeight::Bold);
    assert_eq!(bond.line_weights.left, BondLineWeight::Normal);
    assert_eq!(bond.line_weights.right, BondLineWeight::Normal);

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
    );
    assert_eq!(
        bond.double
            .as_ref()
            .and_then(|double| double.center_exit_side),
        Some(match active_side {
            DoubleBondPlacement::Left => DoubleBondPlacement::Right,
            DoubleBondPlacement::Right => DoubleBondPlacement::Left,
            DoubleBondPlacement::Center => unreachable!(),
        })
    );
    assert_eq!(bond.line_weights.main, BondLineWeight::Normal);
    assert_eq!(
        match active_side {
            DoubleBondPlacement::Left => bond.line_weights.left,
            DoubleBondPlacement::Right => bond.line_weights.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLineWeight::Bold
    );
}

#[test]
fn bold_tool_cycles_plain_center_double_into_bold_states() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(bold_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let first_bold_side = if bond.line_weights.left == BondLineWeight::Bold {
        DoubleBondPlacement::Left
    } else {
        DoubleBondPlacement::Right
    };
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
    );
    assert_eq!(bond.line_weights.main, BondLineWeight::Normal);
    assert_eq!(
        match first_bold_side {
            DoubleBondPlacement::Left => bond.line_weights.left,
            DoubleBondPlacement::Right => bond.line_weights.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLineWeight::Bold
    );

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let exit_side = match first_bold_side {
        DoubleBondPlacement::Left => DoubleBondPlacement::Right,
        DoubleBondPlacement::Right => DoubleBondPlacement::Left,
        DoubleBondPlacement::Center => unreachable!(),
    };
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(exit_side)
    );
    assert_eq!(bond.line_weights.main, BondLineWeight::Bold);
    assert_eq!(bond.line_weights.left, BondLineWeight::Normal);
    assert_eq!(bond.line_weights.right, BondLineWeight::Normal);
}

#[test]
fn click_on_blank_canvas_creates_horizontal_hash_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bold_dashed_bond_tool());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 1);
    assert_eq!(bond.line_styles.main, BondLinePattern::Hash);
    assert_eq!(bond.line_weights.main, BondLineWeight::Normal);
}

#[test]
fn hash_tool_click_on_endpoint_creates_hash_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(bold_dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[1];
    assert_eq!(bond.order, 1);
    assert_eq!(bond.line_styles.main, BondLinePattern::Hash);
    assert_eq!(bond.line_weights.main, BondLineWeight::Normal);
}

#[test]
fn hash_tool_replaces_existing_bond_regardless_of_order() {
    let mut engine = Engine::new();
    engine.set_tool_state(triple_bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    {
        let entry = engine.state().document.editable_fragment().unwrap();
        assert_eq!(entry.fragment.bonds[0].order, 3);
    }

    engine.set_tool_state(bold_dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 1);
    assert!(bond.double.is_none());
    assert!(bond.stereo.is_none());
    assert_eq!(bond.line_styles.main, BondLinePattern::Hash);
    assert_eq!(bond.line_weights.main, BondLineWeight::Normal);
    assert_eq!(bond.line_styles.left, BondLinePattern::Solid);
    assert_eq!(bond.line_weights.left, BondLineWeight::Normal);
}

#[test]
fn click_on_blank_canvas_creates_up_right_wedge_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(wedge_bond_tool());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes[1].position, [FIRST_END_X, FIRST_END_Y]);
    let bond = &entry.fragment.bonds[0];
    let stereo = bond.stereo.as_ref().unwrap();
    assert_eq!(bond.order, 1);
    assert_eq!(stereo.kind, "solid-wedge");
    assert_eq!(stereo.wide_end, "end");
}

#[test]
fn click_on_blank_canvas_creates_up_right_hashed_wedge_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(hashed_wedge_bond_tool());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes[1].position, [FIRST_END_X, FIRST_END_Y]);
    let bond = &entry.fragment.bonds[0];
    let stereo = bond.stereo.as_ref().unwrap();
    assert_eq!(bond.order, 1);
    assert_eq!(stereo.kind, "hashed-wedge");
    assert_eq!(stereo.wide_end, "end");
}

#[test]
fn wedge_tool_replaces_bond_and_toggles_direction() {
    let mut engine = Engine::new();
    engine.set_tool_state(triple_bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(wedge_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let stereo = bond.stereo.as_ref().unwrap();
    assert_eq!(bond.order, 1);
    assert!(bond.double.is_none());
    assert_eq!(stereo.kind, "solid-wedge");
    assert_eq!(stereo.wide_end, "end");

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let stereo = entry.fragment.bonds[0].stereo.as_ref().unwrap();
    assert_eq!(stereo.kind, "solid-wedge");
    assert_eq!(stereo.wide_end, "begin");
}

#[test]
fn hashed_wedge_tool_replaces_bond_and_toggles_direction() {
    let mut engine = Engine::new();
    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(hashed_wedge_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let stereo = bond.stereo.as_ref().unwrap();
    assert_eq!(bond.order, 1);
    assert_eq!(stereo.kind, "hashed-wedge");
    assert_eq!(stereo.wide_end, "end");

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let stereo = entry.fragment.bonds[0].stereo.as_ref().unwrap();
    assert_eq!(stereo.kind, "hashed-wedge");
    assert_eq!(stereo.wide_end, "begin");
}

#[test]
fn all_bond_tools_can_extend_from_existing_endpoint() {
    let tools = [
        bond_tool(),
        double_bond_tool(),
        triple_bond_tool(),
        dashed_bond_tool(),
        dashed_double_bond_tool(),
        bold_bond_tool(),
        bold_dashed_bond_tool(),
        wedge_bond_tool(),
        hashed_wedge_bond_tool(),
    ];

    for tool in tools {
        let mut engine = Engine::new();
        engine.set_tool_state(bond_tool());
        click(&mut engine, px(300.0), px(260.0));

        engine.set_tool_state(tool.clone());
        click(&mut engine, FIRST_END_X, FIRST_END_Y);

        let entry = engine.state().document.editable_fragment().unwrap();
        assert_eq!(entry.fragment.bonds.len(), 2, "{tool:?}");
    }
}

#[test]
fn all_bond_tools_can_focus_existing_triple_bond_center() {
    let tools = [
        bond_tool(),
        double_bond_tool(),
        triple_bond_tool(),
        dashed_bond_tool(),
        dashed_double_bond_tool(),
        bold_bond_tool(),
        bold_dashed_bond_tool(),
        wedge_bond_tool(),
        hashed_wedge_bond_tool(),
    ];

    for tool in tools {
        let mut engine = Engine::new();
        engine.set_tool_state(triple_bond_tool());
        click(&mut engine, px(300.0), px(260.0));

        engine.set_tool_state(tool.clone());
        engine.pointer_move(PointerEvent {
            x: FIRST_CENTER_X,
            y: FIRST_CENTER_Y,
            button: None,
            alt_key: false,
        });

        let hover = engine
            .state()
            .overlay
            .hover_bond_center
            .as_ref()
            .unwrap_or_else(|| panic!("{tool:?} should focus triple-bond center"));
        assert_eq!(hover.order, 3, "{tool:?}");
    }
}

#[test]
fn single_tool_resets_styled_bonds_before_entering_double_cycle() {
    let source_tools = [
        triple_bond_tool(),
        dashed_bond_tool(),
        bold_bond_tool(),
        bold_dashed_bond_tool(),
        wedge_bond_tool(),
        hashed_wedge_bond_tool(),
    ];

    for source in source_tools {
        let mut engine = Engine::new();
        engine.set_tool_state(source.clone());
        click(&mut engine, px(300.0), px(260.0));

        engine.set_tool_state(bond_tool());
        engine.pointer_down(PointerEvent {
            x: FIRST_CENTER_X,
            y: FIRST_CENTER_Y,
            button: Some(0),
            alt_key: false,
        });

        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        assert_eq!(bond.order, 1, "{source:?}");
        assert!(bond.double.is_none(), "{source:?}");
        assert!(bond.stereo.is_none(), "{source:?}");
        assert_eq!(bond.line_styles.main, BondLinePattern::Solid, "{source:?}");
        assert_eq!(bond.line_weights.main, BondLineWeight::Normal, "{source:?}");

        engine.pointer_down(PointerEvent {
            x: FIRST_CENTER_X,
            y: FIRST_CENTER_Y,
            button: Some(0),
            alt_key: false,
        });

        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        assert_eq!(bond.order, 2, "{source:?}");
        assert!(matches!(
            bond.double.as_ref().map(|double| double.placement),
            Some(
                DoubleBondPlacement::Left
                    | DoubleBondPlacement::Right
                    | DoubleBondPlacement::Center
            )
        ));
        assert_eq!(bond.line_styles.main, BondLinePattern::Solid, "{source:?}");
        assert_eq!(bond.line_weights.main, BondLineWeight::Normal, "{source:?}");
    }
}

#[test]
fn double_tool_converts_other_styles_into_expected_double_states() {
    let plain_sources = [
        triple_bond_tool(),
        dashed_bond_tool(),
        bold_dashed_bond_tool(),
        wedge_bond_tool(),
        hashed_wedge_bond_tool(),
    ];

    for source in plain_sources {
        let mut engine = Engine::new();
        engine.set_tool_state(source.clone());
        click(&mut engine, px(300.0), px(260.0));

        engine.set_tool_state(double_bond_tool());
        engine.pointer_down(PointerEvent {
            x: FIRST_CENTER_X,
            y: FIRST_CENTER_Y,
            button: Some(0),
            alt_key: false,
        });

        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        assert_eq!(bond.order, 2, "{source:?}");
        assert!(matches!(
            bond.double.as_ref().map(|double| double.placement),
            Some(
                DoubleBondPlacement::Left
                    | DoubleBondPlacement::Right
                    | DoubleBondPlacement::Center
            )
        ));
        assert_eq!(
            bond.double.as_ref().map(|double| double.frozen),
            Some(false),
            "{source:?}"
        );
        assert!(bond.stereo.is_none(), "{source:?}");
        assert_eq!(bond.line_styles.main, BondLinePattern::Solid, "{source:?}");
        assert_eq!(bond.line_weights.main, BondLineWeight::Normal, "{source:?}");
    }

    let mut engine = Engine::new();
    engine.set_tool_state(bold_bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 2);
    assert!(matches!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right | DoubleBondPlacement::Center)
    ));
    assert_eq!(bond.line_weights.main, BondLineWeight::Bold);
    assert_eq!(bond.line_styles.main, BondLinePattern::Solid);
}

#[test]
fn triple_tool_replaces_existing_style_with_plain_triple() {
    let mut engine = Engine::new();
    engine.set_tool_state(bold_dashed_bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(triple_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 3);
    assert!(bond.double.is_none());
    assert!(bond.stereo.is_none());
    assert_eq!(bond.line_styles.main, BondLinePattern::Solid);
    assert_eq!(bond.line_weights.main, BondLineWeight::Normal);
}

#[test]
fn wedge_tools_preserve_orientation_when_switching_kinds() {
    let mut engine = Engine::new();
    engine.set_tool_state(wedge_bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    {
        let entry = engine.state().document.editable_fragment().unwrap();
        let stereo = entry.fragment.bonds[0].stereo.as_ref().unwrap();
        assert_eq!(stereo.kind, "solid-wedge");
        assert_eq!(stereo.wide_end, "begin");
    }

    engine.set_tool_state(hashed_wedge_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    {
        let entry = engine.state().document.editable_fragment().unwrap();
        let stereo = entry.fragment.bonds[0].stereo.as_ref().unwrap();
        assert_eq!(stereo.kind, "hashed-wedge");
        assert_eq!(stereo.wide_end, "begin");
    }

    engine.set_tool_state(wedge_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let stereo = entry.fragment.bonds[0].stereo.as_ref().unwrap();
    assert_eq!(stereo.kind, "solid-wedge");
    assert_eq!(stereo.wide_end, "begin");
}

#[test]
fn dragged_bond_endpoint_reuses_focused_existing_endpoint() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: FIRST_END_SINGLE_EXTEND_X,
        y: FIRST_END_SINGLE_EXTEND_Y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: FIRST_END_SINGLE_EXTEND_X,
        y: FIRST_END_SINGLE_EXTEND_Y,
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_down(PointerEvent {
        x: FIRST_END_SINGLE_EXTEND_X,
        y: FIRST_END_SINGLE_EXTEND_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: px(304.0),
        y: px(263.0),
        button: None,
        alt_key: false,
    });
    let hover = engine.state().overlay.hover_endpoint.as_ref().unwrap();
    assert_eq!(hover.point.x, FIRST_START_X);
    assert_eq!(hover.point.y, FIRST_START_Y);
    let preview = engine.state().overlay.preview.as_ref().unwrap();
    assert_eq!(preview.end.x, FIRST_START_X);
    assert_eq!(preview.end.y, FIRST_START_Y);
    engine.pointer_up(PointerEvent {
        x: px(304.0),
        y: px(263.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 3);
    assert_eq!(entry.fragment.bonds.len(), 3);
    assert_eq!(
        node_degrees(&engine).values().copied().collect::<Vec<_>>(),
        vec![2, 2, 2]
    );

    engine.pointer_move(PointerEvent {
        x: FIRST_END_X,
        y: px(260.0),
        button: None,
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let closed_bond = entry.fragment.bonds.last().unwrap();
    assert_eq!(closed_bond.order, 2);
    assert!(matches!(
        closed_bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
    ));
    assert_ne!(
        closed_bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
    );
}

#[test]
fn click_extension_reuses_endpoint_at_default_angle() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.add_single_bond(
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(200.0, 200.0),
            label_anchor: None,
        },
        chemsema_engine::Point::new(FIRST_END_SINGLE_EXTEND_X, FIRST_END_SINGLE_EXTEND_Y),
    );

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 4);
    assert_eq!(entry.fragment.bonds.len(), 3);
    let closing = entry.fragment.bonds.last().unwrap();
    assert!(matches!(
        (closing.begin.as_str(), closing.end.as_str()),
        ("n_2", "n_5") | ("n_5", "n_2")
    ));
    assert_eq!(node_degrees(&engine).get("n_2"), Some(&2));
    assert_eq!(node_degrees(&engine).get("n_5"), Some(&2));
}

#[test]
fn select_delete_and_undo_redo_round_trip() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    assert_eq!(fragment_counts(&engine), (2, 1));
    assert!(engine.can_undo());

    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    assert_eq!(engine.state().selection.bonds.len(), 1);

    assert!(engine.delete_selection());
    assert_eq!(fragment_counts(&engine), (0, 0));

    assert!(engine.undo());
    assert_eq!(fragment_counts(&engine), (2, 1));

    assert!(engine.redo());
    assert_eq!(fragment_counts(&engine), (0, 0));
}

#[test]
fn select_delete_atom_removes_attached_bonds_but_keeps_neighbor_atoms() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    click(&mut engine, FIRST_END_X, FIRST_END_Y);
    assert_eq!(fragment_counts(&engine), (3, 2));

    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(FIRST_END_X, FIRST_END_Y), false);
    assert_eq!(engine.state().selection.nodes, vec!["n_2"]);
    assert!(engine.delete_selection());

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.bonds.len(), 0);
    assert_eq!(entry.fragment.nodes.len(), 2);
    assert!(entry.fragment.nodes.iter().all(|node| node.id != "n_2"));
}

#[test]
fn select_copy_and_paste_selected_bond_duplicates_atoms_and_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);

    assert!(engine.copy_selection());
    assert!(engine.paste_clipboard());

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 4);
    assert_eq!(entry.fragment.bonds.len(), 2);
    assert_eq!(engine.state().selection.nodes.len(), 2);
    assert_eq!(engine.state().selection.bonds.len(), 1);
}

#[test]
fn select_cut_stores_bond_then_deletes_and_allows_paste() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);

    assert!(engine.cut_selection());
    assert_eq!(fragment_counts(&engine), (0, 0));
    assert!(engine.paste_clipboard());
    assert_eq!(fragment_counts(&engine), (2, 1));
}

#[test]
fn select_all_after_whole_document_paste_cuts_every_molecule_object() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    assert!(engine.select_all());
    let clipboard_json = engine
        .clipboard_selection_json()
        .expect("clipboard JSON")
        .expect("selected document clipboard");
    assert!(engine
        .paste_clipboard_json(&clipboard_json)
        .expect("clipboard paste"));
    assert!(engine.select_all());
    assert_eq!(engine.state().selection.molecule_objects.len(), 2);

    assert!(engine.cut_selection());
    assert_eq!(
        engine
            .state()
            .document
            .editable_fragments()
            .iter()
            .map(|entry| entry.fragment.bonds.len())
            .sum::<usize>(),
        0
    );
}

#[test]
fn select_cut_undo_redo_is_one_command() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);

    assert!(engine.cut_selection());
    assert_eq!(fragment_counts(&engine), (0, 0));

    assert!(engine.undo());
    assert_eq!(fragment_counts(&engine), (2, 1));

    assert!(engine.redo());
    assert_eq!(fragment_counts(&engine), (0, 0));
}

#[test]
fn select_tool_click_on_text_object_selects_text_box() {
    let mut engine = Engine::new();
    load_text_object_document(&mut engine);
    engine.set_tool_state(select_tool());

    engine.select_at_point(px_point(300.0, 250.0), false);

    assert_eq!(engine.state().selection.text_objects, vec!["obj_text_001"]);
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionTextBox,
            ..
        }
    )));
}

#[test]
fn select_tool_does_not_hover_selected_text_object() {
    let mut engine = Engine::new();
    load_text_object_document(&mut engine);
    engine.set_tool_state(select_tool());

    let point = px_point(300.0, 250.0);
    engine.select_at_point(point, false);
    engine.pointer_move(PointerEvent {
        x: point.x,
        y: point.y,
        button: None,
        alt_key: false,
    });

    assert!(engine.state().overlay.hover_text_box.is_none());
    assert!(!engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::HoverTextBox,
            ..
        }
    )));
}

#[test]
fn select_tool_click_on_label_selects_label_box_not_atom() {
    let mut engine = Engine::new();
    load_label_document(
        &mut engine,
        "CuF3",
        vec![
            rect_polygon(294.0, 256.0, 300.0, 264.0),
            rect_polygon(302.0, 256.0, 308.0, 264.0),
            rect_polygon(310.0, 256.0, 316.0, 264.0),
            rect_polygon(318.0, 256.0, 324.0, 264.0),
        ],
        json!([]),
    );
    engine.set_tool_state(select_tool());

    engine.select_at_point(px_point(305.0, 260.0), false);

    assert_eq!(engine.state().selection.label_nodes, vec!["n1"]);
    assert!(engine.state().selection.nodes.is_empty());
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionTextBox,
            ..
        }
    )));
}

#[test]
fn select_tool_does_not_hover_selected_label_box() {
    let mut engine = Engine::new();
    load_label_document(
        &mut engine,
        "CuF3",
        vec![
            rect_polygon(294.0, 256.0, 300.0, 264.0),
            rect_polygon(302.0, 256.0, 308.0, 264.0),
            rect_polygon(310.0, 256.0, 316.0, 264.0),
            rect_polygon(318.0, 256.0, 324.0, 264.0),
        ],
        json!([]),
    );
    engine.set_tool_state(select_tool());

    let point = px_point(305.0, 260.0);
    engine.select_at_point(point, false);
    engine.pointer_move(PointerEvent {
        x: point.x,
        y: point.y,
        button: None,
        alt_key: false,
    });

    assert!(engine.state().overlay.hover_text_box.is_none());
    assert!(!engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::HoverLabelGlyph | RenderRole::HoverTextBox,
            ..
        }
    )));
}

#[test]
fn select_tool_does_not_hover_selected_bond_or_atom() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    hover(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);

    assert!(engine.state().overlay.hover_bond_center.is_none());
    assert!(!engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Polygon {
            role: RenderRole::HoverBondCenter,
            ..
        }
    )));
    let state_json: serde_json::Value =
        serde_json::from_str(&engine.state_json().expect("state json")).expect("json");
    assert!(state_json["overlay"]["hoverBondCenter"].is_null());
    assert!(state_json["overlay"].get("hoverBondTarget").is_none());

    engine.select_at_point(Point::new(FIRST_END_X, FIRST_END_Y), false);
    hover(&mut engine, FIRST_END_X, FIRST_END_Y);

    assert!(engine.state().overlay.hover_endpoint.is_none());
    let state_json: serde_json::Value =
        serde_json::from_str(&engine.state_json().expect("state json")).expect("json");
    assert!(state_json["overlay"]["hoverEndpoint"].is_null());
    assert!(!engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle {
            role: RenderRole::HoverEndpoint,
            ..
        }
    )));
}

#[test]
fn select_tool_click_on_endpoint_selects_atom_box() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(FIRST_END_X, FIRST_END_Y), false);

    assert_eq!(engine.state().selection.nodes, vec!["n_2"]);
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionNode,
            ..
        }
    )));
}

#[test]
fn select_tool_box_selecting_endpoint_matches_click_endpoint_affordance() {
    let mut clicked = Engine::new();
    clicked.set_tool_state(bond_tool());
    click(&mut clicked, px(300.0), px(260.0));
    clicked.set_tool_state(select_tool());
    clicked.select_at_point(Point::new(FIRST_END_X, FIRST_END_Y), false);

    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());

    engine.select_in_rect(
        Point::new(FIRST_END_X - px(2.0), FIRST_END_Y - px(2.0)),
        Point::new(FIRST_END_X + px(2.0), FIRST_END_Y + px(2.0)),
        false,
    );

    assert_eq!(engine.state().selection.nodes, vec!["n_2"]);
    assert!(engine.state().selection.bonds.is_empty());
    assert_eq!(
        clicked.state().selection.nodes,
        engine.state().selection.nodes
    );
    assert_eq!(
        clicked.state().selection.bonds,
        engine.state().selection.bonds
    );
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionNode,
            ..
        }
    )));
    assert_eq!(selection_bond_dots(&clicked).len(), 1);
    assert_eq!(selection_bond_dots(&engine).len(), 1);
}

#[test]
fn select_tool_click_on_bond_does_not_render_outer_region_box() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);

    let (_, _, selection_width, selection_height) = selection_bond_rect(&engine);
    assert!(selection_width >= px(3.0));
    assert!(selection_height >= px(3.0));
    let render_list = engine.render_list();
    assert!(render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionBond,
            ..
        }
    )));
    assert!(!render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionBox,
            ..
        }
    )));
    let bond_dots = selection_bond_dots(&engine);
    assert_eq!(bond_dots.len(), 1);
    assert!(matches!(
        &bond_dots[0],
        RenderPrimitive::Circle {
            stroke,
            stroke_width,
            ..
        } if stroke == "none" && *stroke_width == 0.0
    ));
}

#[test]
fn switching_to_select_selects_latest_changed_graphic_or_molecule_component() {
    let mut shape_engine = Engine::new();
    shape_engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    shape_engine.pointer_down(PointerEvent {
        x: 20.0,
        y: 20.0,
        button: Some(0),
        alt_key: false,
    });
    shape_engine.pointer_move(PointerEvent {
        x: 60.0,
        y: 44.0,
        button: None,
        alt_key: false,
    });
    shape_engine.pointer_up(PointerEvent {
        x: 60.0,
        y: 44.0,
        button: Some(0),
        alt_key: false,
    });
    let shape_id = shape_engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "shape")
        .expect("shape object should exist")
        .id
        .clone();
    shape_engine.set_tool_state(select_tool());
    assert_eq!(shape_engine.state().selection.arrow_objects, vec![shape_id]);

    let mut arrow_engine = Engine::new();
    arrow_engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        ..ToolState::default()
    });
    arrow_engine.pointer_down(PointerEvent {
        x: 20.0,
        y: 20.0,
        button: Some(0),
        alt_key: false,
    });
    arrow_engine.pointer_move(PointerEvent {
        x: 70.0,
        y: 20.0,
        button: None,
        alt_key: false,
    });
    arrow_engine.pointer_up(PointerEvent {
        x: 70.0,
        y: 20.0,
        button: Some(0),
        alt_key: false,
    });
    let arrow_id = arrow_engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow object should exist")
        .id
        .clone();
    arrow_engine.set_tool_state(select_tool());
    assert_eq!(arrow_engine.state().selection.arrow_objects, vec![arrow_id]);

    let mut bond_engine = Engine::new();
    bond_engine.set_tool_state(bond_tool());
    click(&mut bond_engine, px(300.0), px(260.0));
    bond_engine.set_tool_state(select_tool());
    assert_eq!(bond_engine.state().selection.nodes.len(), 2);
    assert_eq!(bond_engine.state().selection.bonds.len(), 1);
}
