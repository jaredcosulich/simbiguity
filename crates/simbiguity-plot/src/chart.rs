use crate::{axes::Axes, series::Series};
use alloc::string::String;
use alloc::vec::Vec;
use simbiguity_render::{Command, CommandBuffer, Mesh, Program, Uniform};

/// A chart combining axes and multiple data series, projecting data space to pixel space.
///
/// A chart orchestrates the mapping from data domain to screen coordinates using `Axes`.
/// The `draw` function downsamples data, builds line geometry, and returns
/// ready-to-upload `Mesh` objects alongside the formatted tick labels in pixel space.
pub struct Chart<'a> {
    /// Defines the data space to pixel space mapping.
    pub axes: Axes,
    /// List of data series and their RGBA colors.
    pub series: Vec<(&'a Series, [f32; 4])>,
}

impl<'a> Chart<'a> {
    /// Maps each series through Axes, downsamples, builds line meshes, and appends draw commands.
    pub fn draw(
        &self,
        cmd: &mut CommandBuffer,
        start_mesh_id: u32,
    ) -> (Vec<Mesh>, Vec<(String, f64, f64)>) {
        let mut meshes = Vec::new();
        let mut labels = Vec::new();
        let mut next_mesh_id = start_mesh_id;

        cmd.push(&Command::Viewport(
            self.axes.px_w as u32,
            self.axes.px_h as u32,
        ));
        cmd.push(&Command::SetUniformMat4(
            Uniform::ViewProj,
            simbiguity_render::camera::orthographic_projection(
                self.axes.px_x,
                self.axes.px_x + self.axes.px_w,
                self.axes.px_y + self.axes.px_h,
                self.axes.px_y,
                -1.0,
                1.0,
            )
            .m
            .map(|x| x as f32),
        ));
        cmd.push(&Command::SetUniformMat4(
            Uniform::Model,
            simbiguity_math::Mat4::IDENTITY.m.map(|x| x as f32),
        ));

        let (ticks_x, step_x) =
            crate::ticks::NiceTicks::compute(self.axes.x_min, self.axes.x_max, 5);
        let (ticks_y, step_y) =
            crate::ticks::NiceTicks::compute(self.axes.y_min, self.axes.y_max, 5);

        let mut grid_mesh = Mesh::default();

        for &x in &ticks_x {
            let (px_x, px_y_min) = self.axes.data_to_px(x, self.axes.y_min);
            let (_, px_y_max) = self.axes.data_to_px(x, self.axes.y_max);

            let grid_pts = [(px_x, px_y_min), (px_x, px_y_max)];
            let m = crate::line::build_line_mesh(&grid_pts, 1.0);

            let offset = grid_mesh.vertex_count() as u32;
            grid_mesh.vertices.extend_from_slice(&m.vertices);
            grid_mesh
                .indices
                .extend(m.indices.iter().map(|i| i + offset));

            labels.push((
                crate::ticks::NiceTicks::format_tick(x, step_x),
                px_x,
                px_y_min + 15.0,
            ));
        }

        for &y in &ticks_y {
            let (px_x_min, px_y) = self.axes.data_to_px(self.axes.x_min, y);
            let (px_x_max, _) = self.axes.data_to_px(self.axes.x_max, y);

            let grid_pts = [(px_x_min, px_y), (px_x_max, px_y)];
            let m = crate::line::build_line_mesh(&grid_pts, 1.0);

            let offset = grid_mesh.vertex_count() as u32;
            grid_mesh.vertices.extend_from_slice(&m.vertices);
            grid_mesh
                .indices
                .extend(m.indices.iter().map(|i| i + offset));

            labels.push((
                crate::ticks::NiceTicks::format_tick(y, step_y),
                px_x_min - 15.0,
                px_y,
            ));
        }

        if grid_mesh.vertex_count() > 0 {
            cmd.push(&Command::SetProgram(Program::Unlit));
            cmd.push(&Command::SetUniformVec4(
                Uniform::Color,
                [0.8, 0.8, 0.8, 1.0],
            ));
            cmd.push(&Command::BindMesh(next_mesh_id));
            cmd.push(&Command::DrawIndexed {
                count: grid_mesh.indices.len() as u32,
                first: 0,
            });
            meshes.push(grid_mesh);
            next_mesh_id += 1;
        }

        for (series, color) in &self.series {
            let mut mapped_points = Vec::new();

            let mut current_col = None;
            let mut col_min = f64::MAX;
            let mut col_max = f64::MIN;

            for &(time, value) in series.iter() {
                let (px_x, px_y) = self.axes.data_to_px(time, value);
                let col = simbiguity_math::float::floor(px_x) as isize;

                if let Some(c) = current_col {
                    if c == col {
                        col_min = col_min.min(px_y);
                        col_max = col_max.max(px_y);
                    } else {
                        if col_min < col_max {
                            mapped_points.push((c as f64 + 0.5, col_min));
                            mapped_points.push((c as f64 + 0.5, col_max));
                        } else {
                            mapped_points.push((c as f64 + 0.5, col_min));
                        }
                        current_col = Some(col);
                        col_min = px_y;
                        col_max = px_y;
                    }
                } else {
                    current_col = Some(col);
                    col_min = px_y;
                    col_max = px_y;
                }
            }

            if let Some(c) = current_col {
                if col_min < col_max {
                    mapped_points.push((c as f64 + 0.5, col_min));
                    mapped_points.push((c as f64 + 0.5, col_max));
                } else {
                    mapped_points.push((c as f64 + 0.5, col_min));
                }
            }

            let mesh = crate::line::build_line_mesh(&mapped_points, 2.0);
            if mesh.vertex_count() > 0 {
                cmd.push(&Command::SetProgram(Program::Unlit));
                cmd.push(&Command::SetUniformVec4(Uniform::Color, *color));
                cmd.push(&Command::BindMesh(next_mesh_id));
                cmd.push(&Command::DrawIndexed {
                    count: mesh.indices.len() as u32,
                    first: 0,
                });
                meshes.push(mesh);
                next_mesh_id += 1;
            }
        }

        (meshes, labels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simbiguity_render::{Command, CommandBuffer, Uniform};

    fn sample_chart<'a>(s: &'a Series) -> Chart<'a> {
        Chart {
            axes: crate::axes::Axes {
                x_min: 0.0,
                x_max: 1.0,
                y_min: 0.0,
                y_max: 1.0,
                px_x: 0.0,
                px_y: 0.0,
                px_w: 200.0,
                px_h: 100.0,
            },
            series: alloc::vec![(s, [1.0, 0.0, 0.0, 1.0])],
        }
    }

    /// The GPU draws a triangle only after it's told how to map positions to the
    /// screen (the view-projection matrix). A chart that draws its lines without
    /// first setting that matrix produces nothing visible, however correct its
    /// data is.
    #[test]
    fn chart_sets_view_projection_before_drawing() {
        let mut s = Series::new(8);
        s.push(0.0, 0.0);
        s.push(1.0, 1.0);
        let chart = sample_chart(&s); // [0,1]×[0,1] on 200×100 px; settle the constructor at execution
        let mut cmd = CommandBuffer::new();
        chart.draw(&mut cmd, 0);
        let cmds = CommandBuffer::decode(cmd.words()).unwrap();
        let first_draw = cmds
            .iter()
            .position(|c| matches!(c, Command::DrawIndexed { .. }))
            .unwrap();
        assert!(cmds[..first_draw]
            .iter()
            .any(|c| matches!(c, Command::SetUniformMat4(Uniform::ViewProj, _))));
    }

    /// Verifies that downsampling based on pixel width preserves local max/min spikes.
    #[test]
    fn downsampling_preserves_spikes() {
        let mut s = Series::new(10005);
        for i in 0..10000 {
            s.push(i as f64, 0.0);
        }
        s.push(5000.0, 1.0); // spike
        let mut chart = sample_chart(&s);
        chart.axes.x_max = 10000.0;
        let mut cmd = CommandBuffer::new();
        let (meshes, _) = chart.draw(&mut cmd, 0);
        let mesh = meshes.last().unwrap();
        // downsampling: max 4 vertices (2 points) per pixel width. 200 px = max 800 vertices
        assert!(mesh.vertex_count() <= 800);
        let max_y = mesh
            .vertices
            .chunks(6)
            .map(|v| v[1])
            .fold(f32::INFINITY, f32::min);
        assert!(max_y < 50.0); // spike reaches 0 in pixel space (top)
    }
}
