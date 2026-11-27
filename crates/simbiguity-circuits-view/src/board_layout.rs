use alloc::vec::Vec;
use simbiguity_circuits::ComponentId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    UnplacedComponent,
    CellOccupied,
    DanglingWire,
    RouteTooShort,
    BufferTooShort,
    UnknownComponentKind,
    BadNodeId,
}

impl LayoutError {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UnplacedComponent => "a component left unplaced",
            Self::CellOccupied => "two parts in one cell",
            Self::DanglingWire => "a wire endpoint that isn't on a terminal",
            Self::RouteTooShort => "a route with fewer than 2 points",
            Self::BufferTooShort => "buffer too short",
            Self::UnknownComponentKind => "unknown component kind",
            Self::BadNodeId => "bad node id",
        }
    }
}

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
    pub fn validate(&self, circuit: &simbiguity_circuits::Circuit) -> Result<(), LayoutError> {
        let mut placed = alloc::vec::Vec::new();
        let mut cells = alloc::vec::Vec::new();
        for p in &self.parts {
            placed.push(p.component);
            if cells.contains(&p.cell) {
                return Err(LayoutError::CellOccupied);
            }
            cells.push(p.cell);
        }
        for w in &self.wires {
            placed.push(w.component);
            if w.points.len() < 2 {
                return Err(LayoutError::RouteTooShort);
            }
            let start = w.points.first().ok_or(LayoutError::RouteTooShort)?;
            let end = w.points.last().ok_or(LayoutError::RouteTooShort)?;
            let start_ok = self.parts.iter().any(|p| &p.cell == start);
            let end_ok = self.parts.iter().any(|p| &p.cell == end);
            if !start_ok || !end_ok {
                return Err(LayoutError::DanglingWire);
            }
        }
        placed.sort();
        placed.dedup();
        if placed.len() != circuit.parts().len() {
            return Err(LayoutError::UnplacedComponent);
        }
        Ok(())
    }

    pub fn unpack(words: &[u32]) -> Result<(simbiguity_circuits::Circuit, Self), LayoutError> {
        if words.len() < 2 {
            return Err(LayoutError::BufferTooShort);
        }
        let node_count = words[0] as usize;
        let num_parts = words[1] as usize;
        let mut c = simbiguity_circuits::Circuit::new();
        let mut nodes = alloc::vec::Vec::with_capacity(node_count);
        for _ in 0..node_count {
            nodes.push(c.node());
        }

        let mut parts = alloc::vec::Vec::new();
        let mut wires = alloc::vec::Vec::new();
        let mut pos = 2;

        let f64_from = |words: &[u32], i: usize| -> f64 {
            f64::from_bits((words[i] as u64) | ((words[i + 1] as u64) << 32))
        };

        for _ in 0..num_parts {
            if pos >= words.len() {
                return Err(LayoutError::BufferTooShort);
            }
            let kind = words[pos];
            if pos + 3 > words.len() {
                return Err(LayoutError::BufferTooShort);
            }
            let a_idx = words[pos + 1] as usize;
            let b_idx = words[pos + 2] as usize;
            if a_idx >= node_count || b_idx >= node_count {
                return Err(LayoutError::BadNodeId);
            }
            let a = nodes[a_idx];
            let b = nodes[b_idx];

            let rotation = match words.get(pos + 5).copied().unwrap_or(0) {
                0 => Rotation::Deg0,
                1 => Rotation::Deg90,
                2 => Rotation::Deg180,
                _ => Rotation::Deg270,
            };

            match kind {
                1 => {
                    if pos + 8 > words.len() {
                        return Err(LayoutError::BufferTooShort);
                    }
                    let emf = f64_from(words, pos + 6);
                    let cid = c.add_battery(a, b, emf);
                    parts.push(PartPlacement {
                        component: cid,
                        cell: (words[pos + 3] as i32, words[pos + 4] as i32),
                        rotation,
                    });
                    pos += 8;
                }
                2 => {
                    if pos + 7 > words.len() {
                        return Err(LayoutError::BufferTooShort);
                    }
                    let cid = c.add_switch(a, b, words[pos + 6] != 0);
                    parts.push(PartPlacement {
                        component: cid,
                        cell: (words[pos + 3] as i32, words[pos + 4] as i32),
                        rotation,
                    });
                    pos += 7;
                }
                3 => {
                    if pos + 10 > words.len() {
                        return Err(LayoutError::BufferTooShort);
                    }
                    let volts = f64_from(words, pos + 6);
                    let watts = f64_from(words, pos + 8);
                    let cid = c.add_bulb(a, b, volts, watts);
                    parts.push(PartPlacement {
                        component: cid,
                        cell: (words[pos + 3] as i32, words[pos + 4] as i32),
                        rotation,
                    });
                    pos += 10;
                }
                4 => {
                    if pos + 4 > words.len() {
                        return Err(LayoutError::BufferTooShort);
                    }
                    let pts = words[pos + 3] as usize;
                    if pos + 4 + pts * 2 > words.len() {
                        return Err(LayoutError::BufferTooShort);
                    }
                    let cid = c.connect(a, b);
                    let mut points = alloc::vec::Vec::new();
                    for i in 0..pts {
                        points.push((words[pos + 4 + i * 2] as i32, words[pos + 5 + i * 2] as i32));
                    }
                    wires.push(WireRoute {
                        component: cid,
                        points,
                    });
                    pos += 4 + pts * 2;
                }
                _ => return Err(LayoutError::UnknownComponentKind),
            }
        }
        let layout = BoardLayout { parts, wires };
        layout.validate(&c)?;
        Ok((c, layout))
    }

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
