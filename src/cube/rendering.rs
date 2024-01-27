use ndarray::{arr1, arr2, Array, Array1, Array2, ArrayView2};

use super::{Color, Cube, CubeAxis, FaceDir};

#[derive(Debug, Clone)]
struct Vertex {
    coordinate: Array1<f32>,
}
impl Vertex {
    fn new(x: f32, y: f32, z: f32) -> Vertex {
        Vertex {
            coordinate: arr1(&[x, y, z]),
        }
    }
    fn translate(&mut self, axis: CubeAxis, amount: f32) {
        match axis {
            CubeAxis::X => self.coordinate[0] += amount,
            CubeAxis::Y => self.coordinate[1] += amount,
            CubeAxis::Z => self.coordinate[2] += amount,
        }
    }
    fn transform(&mut self, matrix: ArrayView2<f32>) {
        self.coordinate = matrix.dot(&self.coordinate);
    }
    fn to_img_coordinates(&mut self, x_scale: f32, y_scale: f32, img_w: usize, img_h: usize) {
        self.coordinate[0] = self.coordinate[0] * x_scale + img_w as f32 / 2.0;
        self.coordinate[1] = -self.coordinate[1] * y_scale + img_h as f32 / 2.0;
    }
    fn get_proj(&self) -> (f32, f32) {
        (self.coordinate[0], self.coordinate[1])
    }
    fn z(&self) -> f32 {
        self.coordinate[2]
    }
}

struct BoundingBoxIterator {
    x: usize,
    y: usize,
    min_x: usize,
    max_x: usize,
    max_y: usize,
}
impl BoundingBoxIterator {
    fn new(min_x: usize, min_y: usize, max_x: usize, max_y: usize) -> BoundingBoxIterator {
        BoundingBoxIterator {
            x: min_x,
            y: min_y,
            min_x,
            max_x,
            max_y,
        }
    }
}
impl Iterator for BoundingBoxIterator {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.y <= self.max_y {
            let (x, y) = (self.x, self.y);
            self.x += 1;
            if self.x > self.max_x {
                self.x = self.min_x;
                self.y += 1;
            }
            Some((x, y))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
struct Quad {
    vertices: [Vertex; 4],
    color: Color,
}
impl Quad {
    fn new(vertices: [Vertex; 4], color: Color) -> Quad {
        Quad { vertices, color }
    }

    fn transform(&mut self, matrix: ArrayView2<f32>) {
        for vertex in self.vertices.iter_mut() {
            vertex.transform(matrix);
        }
    }

    fn to_img_coordinates(&mut self, x_scale: f32, y_scale: f32, img_w: usize, img_h: usize) {
        for vertex in self.vertices.iter_mut() {
            vertex.to_img_coordinates(x_scale, y_scale, img_w, img_h);
        }
    }

    fn iter_proj_bounding_box(&self) -> BoundingBoxIterator {
        let (mut min_x, mut min_y) = (f32::INFINITY, f32::INFINITY);
        let (mut max_x, mut max_y) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        for vertex in self.vertices.iter() {
            let (x, y) = vertex.get_proj();
            max_x = f32::max(x, max_x);
            min_x = f32::min(x, min_x);
            max_y = f32::max(y, max_y);
            min_y = f32::min(y, min_y);
        }
        BoundingBoxIterator::new(
            min_x as usize,
            min_y as usize,
            max_x as usize,
            max_y as usize,
        )
    }

    fn is_point_in_proj(&self, x: f32, y: f32) -> bool {
        let mut count = 0;
        for i in 0..4 {
            let (x1, y1) = self.vertices[i].get_proj();
            let (x2, y2) = self.vertices[(i + 1) % 4].get_proj();

            // check if ray going right (x positive) from (x, y) intersects with the line segment
            if y1 == y2 {
                continue;
            }
            if f32::min(y1, y2) < y && y < f32::max(y1, y2) {
                let x_intersect = (x2 - x1) * (y - y1) / (y2 - y1) + x1;
                if x < x_intersect {
                    count += 1;
                }
            }
        }
        count % 2 == 1
    }

    /// Returns the -z value of the middle of the quad
    fn get_depth(&self) -> f32 {
        let (mut max_z, mut min_z) = (f32::NEG_INFINITY, f32::INFINITY);
        for vertex in self.vertices.iter() {
            let z = vertex.z();
            max_z = f32::max(max_z, z);
            min_z = f32::min(min_z, z);
        }
        -(max_z + min_z) / 2.0 // the higher the z, the lower the depth, so flip the sign
    }
}
pub struct CubeRender {
    pitch: f32,
    yaw: f32,
    x_scale: f32,
    y_scale: f32,
    img_w: usize,
    img_h: usize,
    quads: Vec<Quad>,
}
impl CubeRender {
    const INIT_PITCH: f32 = 0.0;
    const INIT_YAW: f32 = 0.0;
    pub fn new(cube: &Cube, x_scale: f32, y_scale: f32, img_w: usize, img_h: usize) -> CubeRender {
        let mut new_cr = CubeRender {
            pitch: CubeRender::INIT_PITCH,
            yaw: CubeRender::INIT_YAW,
            x_scale,
            y_scale,
            img_w,
            img_h,
            quads: Vec::new(),
        };
        new_cr.update_colors(cube);
        new_cr
    }

    pub fn update_colors(&mut self, cube: &Cube) {
        self.quads = Vec::with_capacity(cube.size.pow(3) * 6);
        for face_dir in cube.dir_order.iter() {
            for ((y, x), color) in cube.get_face(face_dir).colors.indexed_iter() {
                // calculate the first coordinate of the quad
                // the down left back corner is at 0, 0, 0
                let (px, py, pz) = match face_dir {
                    FaceDir::Up => (x, cube.size, y),
                    FaceDir::Down => (x, 0, cube.size - y),
                    FaceDir::Right => (cube.size, cube.size - y, cube.size - x),
                    FaceDir::Left => (0, cube.size - y, x),
                    FaceDir::Front => (x, cube.size - y, cube.size),
                    FaceDir::Back => (cube.size - x, cube.size - y, 0),
                };
                let (px, py, pz) = (px as f32, py as f32, pz as f32);

                // center at the origin
                let half_cube: f32 = cube.size as f32 / 2.0;
                let (px, py, pz) = (px - half_cube, py - half_cube, pz - half_cube);

                // generate the four vertices of the quad, order based on axis order
                let mut vertices = [
                    Vertex::new(px, py, pz),
                    Vertex::new(px, py, pz),
                    Vertex::new(px, py, pz),
                    Vertex::new(px, py, pz),
                ];
                let [first_axis_dir, second_axis_dir] = Cube::get_axes_order(face_dir);
                let first_axis = first_axis_dir.get_axis();
                let second_axis = second_axis_dir.get_axis();
                let first_axis_amt = if first_axis_dir.is_positive() {
                    1.0
                } else {
                    -1.0
                };
                let second_axis_amt = if second_axis_dir.is_positive() {
                    1.0
                } else {
                    -1.0
                };
                vertices[1].translate(first_axis, first_axis_amt);
                vertices[2].translate(first_axis, first_axis_amt);
                vertices[2].translate(second_axis, second_axis_amt);
                vertices[3].translate(second_axis, second_axis_amt);

                // add new square to the array of quads
                self.quads.push(Quad::new(vertices, *color));
            }
        }
        let pitch_matrix = CubeRender::pitch_matrix(self.pitch);
        let yaw_matrix = CubeRender::yaw_matrix(self.yaw);
        let rotation_matrix = pitch_matrix.dot(&yaw_matrix);
        for square in self.quads.iter_mut() {
            square.transform(rotation_matrix.view());
        }
    }

    pub fn render_cube(&self) {
        // create img structures
        let mut img_arr = Array::from_elem((self.img_h, self.img_w), None);
        //stores the depth of the pixel, to avoid squares from behind being drawn on top
        let mut img_depth = Array::from_elem((self.img_h, self.img_w), f32::INFINITY);

        // render each square to `img_arr`
        for square in self.quads.iter() {
            let mut img_square = square.clone();
            img_square.to_img_coordinates(self.x_scale, self.y_scale, self.img_w, self.img_h);
            for (x, y) in img_square.iter_proj_bounding_box() {
                if x >= self.img_w || y >= self.img_h {
                    continue;
                }
                if img_square.is_point_in_proj(x as f32, y as f32) {
                    let depth = square.get_depth();
                    if img_depth[[y, x]] > depth {
                        img_arr[[y, x]] = Some(square.color);
                        img_depth[[y, x]] = depth;
                    }
                }
            }
        }

        for row in img_arr.outer_iter() {
            for e in row.iter() {
                match e {
                    None => print!(" "),
                    Some(color) => print!("{}", color),
                }
            }
            println!();
        }
    }

    pub fn rotate_pitch(&mut self, dp: f32) {
        self.pitch += dp;
        let pitch_matrix = CubeRender::pitch_matrix(dp);
        for square in self.quads.iter_mut() {
            square.transform(pitch_matrix.view());
        }
    }
    pub fn rotate_yaw(&mut self, dy: f32) {
        self.yaw += dy;

        let yaw_matrix = CubeRender::yaw_matrix(dy);
        for square in self.quads.iter_mut() {
            square.transform(yaw_matrix.view());
        }
    }

    fn pitch_matrix(pitch: f32) -> Array2<f32> {
        arr2(&[
            [1.0, 0.0, 0.0],
            [0.0, pitch.cos(), pitch.sin()],
            [0.0, -pitch.sin(), pitch.cos()],
        ])
    }
    fn yaw_matrix(yaw: f32) -> Array2<f32> {
        arr2(&[
            [yaw.cos(), 0.0, yaw.sin()],
            [0.0, 1.0, 0.0],
            [-yaw.sin(), 0.0, yaw.cos()],
        ])
    }
}
