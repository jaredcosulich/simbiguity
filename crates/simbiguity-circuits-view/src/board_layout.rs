#![allow(missing_docs)]
use alloc::vec::Vec;
use simbiguity_circuits::ComponentId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartPlacement {
    pub component: ComponentId,
    pub cell: (i32, i32),
    pub rotation: Rotation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireRoute {
    pub component: ComponentId,
    pub points: Vec<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardLayout {
    pub parts: Vec<PartPlacement>,
    pub wires: Vec<WireRoute>,
}

impl BoardLayout {
    pub fn hello_layout(
        battery: ComponentId,
        switch: ComponentId,
        bulb: ComponentId,
        wires: [ComponentId; 3],
    ) -> Self {
        Self {
            parts: alloc::vec![
                PartPlacement {
                    component: battery,
                    cell: (-1, 0),
                    rotation: Rotation::Deg0
                },
                PartPlacement {
                    component: switch,
                    cell: (0, -1),
                    rotation: Rotation::Deg0
                },
                PartPlacement {
                    component: bulb,
                    cell: (1, 0),
                    rotation: Rotation::Deg0
                },
            ],
            wires: alloc::vec![
                WireRoute {
                    component: wires[0],
                    points: alloc::vec![(-1, 0), (0, -1)]
                },
                WireRoute {
                    component: wires[1],
                    points: alloc::vec![(0, -1), (1, 0)]
                },
                WireRoute {
                    component: wires[2],
                    points: alloc::vec![(1, 0), (-1, 0)]
                },
            ],
        }
    }
}
